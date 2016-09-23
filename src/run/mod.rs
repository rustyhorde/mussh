use {MusshResult, STDERR_SW, STDOUT_SW};
use clap::{App, Arg, ArgMatches};
use config::{DOT_DIR, MusshToml, STDERR_FILE, STDOUT_FILE};
use error::MusshErr;
use slog::{DrainExt, Level, Logger, duplicate, level_filter};
use slog_stream::{async_stream, stream};
use slog_term::{self, ColorDecorator, Format, FormatMode};
use ssh2::Session;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Instant;

fn setup_hostnames(config: &MusshToml, matches: &ArgMatches) -> MusshResult<Vec<String>> {
    let stdout = Logger::root(STDOUT_SW.drain().fuse(), o!());
    let mut hostnames = Vec::new();
    if let Some(hosts_arg) = matches.value_of("hosts") {
        if let Some(hosts) = config.hostlist() {
            for (name, host_config) in hosts {
                if name == hosts_arg {
                    hostnames = host_config.hostnames().clone();
                    trace!(stdout, "setup_hostnames"; "hostnames" => format!("{:?}", hostnames));
                    break;
                }
            }
        }
    } else {
        return Err(MusshErr::InvalidHosts);
    }

    if hostnames.is_empty() {
        Err(MusshErr::InvalidHosts)
    } else {
        Ok(hostnames)
    }
}

fn setup_command(config: &MusshToml, matches: &ArgMatches) -> MusshResult<String> {
    let stdout = Logger::root(STDOUT_SW.drain().fuse(), o!());
    let mut cmd = String::new();
    if let Some(cmd_arg) = matches.value_of("command") {
        if let Some(cmds) = config.cmd() {
            for (name, command) in cmds {
                if name == cmd_arg {
                    cmd = command.command().clone();
                    trace!(stdout, "setup_command"; "command" => cmd);
                    break;
                }
            }
        }
    } else {
        return Err(MusshErr::InvalidCmd("arg not matched".to_string()));
    }

    if cmd.is_empty() {
        Err(MusshErr::InvalidCmd("empty command".to_string()))
    } else {
        Ok(cmd)
    }
}

type ConfigTuple = (String, String, u16, Option<String>, Option<HashMap<String, String>>);

fn setup_host(config: &MusshToml, hostname: &str) -> MusshResult<ConfigTuple> {
    if let Some(hosts) = config.hosts() {
        if let Some(host) = hosts.get(hostname) {
            let username = host.username();
            let hn = host.hostname();
            let pem = if let Some(pem) = host.pem() {
                Some(pem.clone())
            } else {
                None
            };
            let port = host.port().unwrap_or(22);
            let alias = if let Some(al) = host.alias() {
                Some(al.clone())
            } else {
                None
            };
            Ok((username.clone(), hn.clone(), port, pem, alias))
        } else {
            // TODO: fix this error
            Err(MusshErr::Unknown)
        }
    } else {
        // TODO: fix this error
        Err(MusshErr::Unknown)
    }
}

fn execute<A: ToSocketAddrs>(hostname: String,
                             command: String,
                             username: String,
                             pem: Option<String>,
                             host: A)
                             -> MusshResult<()> {
    let stdout = Logger::root(STDOUT_SW.drain().fuse(), o!());
    let stderr = Logger::root(STDERR_SW.drain().fuse(), o!());

    let mut host_file_path = if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(DOT_DIR);
        home_dir
    } else {
        PathBuf::new()
    };

    host_file_path.push(hostname.clone());
    host_file_path.set_extension("log");

    let outfile = OpenOptions::new().create(true).append(true).open(&host_file_path)?;
    let fmt = Format::new(FormatMode::Full, ColorDecorator::new_plain());
    let file_async = level_filter(Level::Trace, stream(outfile, fmt));
    let file_logger = Logger::root(file_async.fuse(), o!());
    let timer = Instant::now();

    if &hostname == "localhost" {
        let mut cmd = Command::new("/usr/bin/fish");
        cmd.arg("-c");
        cmd.arg(command);
        cmd.stdout(Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            let stdout_reader = BufReader::new(child.stdout.take().expect(""));
            let hn = hostname.clone();
            for line in stdout_reader.lines() {
                trace!(file_logger, "execute"; "hostname" => hn, "line" => line.expect(""));
            }

            match child.wait() {
                Ok(status) => {
                    if let Some(code) = status.code() {
                        info!(
                            stdout,
                            "execute";
                            "hostname" => hn,
                            "code" => code,
                            "duration" => timer.elapsed().as_secs()
                        );
                    } else {
                        error!(stderr, "execute"; "hostname" => hn, "error" => "No exit code");
                    }
                }
                Err(e) => {
                    error!(stderr, "execute"; "hostname" => hn, "error" => format!("{}", e));
                }
            }
        }
    } else if let Some(mut sess) = Session::new() {
        let tcp = TcpStream::connect(host)?;
        sess.handshake(&tcp)?;
        if let Some(pem) = pem {
            sess.userauth_pubkey_file(&username, None, Path::new(&pem), None)?;
        } else {
            trace!(stdout, "execute"; "message" => "Agent Auth Setup", "username" => username);
            let mut agent = sess.agent()?;
            agent.connect()?;
            agent.list_identities()?;
            for identity in agent.identities() {
                if let Ok(ref id) = identity {
                    if let Ok(_) = agent.userauth(&username, id) {
                        break;
                    }
                }
            }
            agent.disconnect()?;
            // sess.userauth_agent(&username)?;
        }

        if sess.authenticated() {
            trace!(stdout, "execute"; "message" => "Authenticated");
            let mut channel = sess.channel_session()?;
            channel.exec(&command)?;
            let hn = hostname.clone();

            {
                let stdout_stream = channel.stream(0);
                let stdout_reader = BufReader::new(stdout_stream);

                for line in stdout_reader.lines() {
                    trace!(file_logger, "execute"; "hostname" => hn, "line" => line.expect(""));
                }
            }

            match channel.exit_status() {
                Ok(code) => {
                    if code == 0 {
                        info!(
                            stdout,
                            "execute";
                            "hostname" => hn,
                            "code" => code,
                            "duration" => timer.elapsed().as_secs()
                        );
                    } else {
                        error!(stderr, "execute"; "hostname" => hn, "code" => code);
                    }
                }
                Err(e) => {
                    error!(stderr, "execute"; "hostname" => hn, "error" => format!("{}", e));
                }
            }
        } else {
            return Err(MusshErr::Auth);
        }
    } else {
        return Err(MusshErr::InvalidSshSession);
    }

    Ok(())
}

fn multiplex(config: MusshToml, matches: ArgMatches) -> MusshResult<()> {
    let hostnames = setup_hostnames(&config, &matches)?;
    let cmd = setup_command(&config, &matches)?;
    let mut children = Vec::new();

    for hostname in hostnames.into_iter() {
        let t_hostname = hostname.clone();
        let (username, hn, port, pem, alias) = setup_host(&config, &t_hostname)?;

        let t_cmd = if let Some(alias_map) = alias {
            if let Some(cmd_arg) = matches.value_of("command") {
                if let Some(alias) = alias_map.get(cmd_arg) {
                    if let Some(cmds) = config.cmd() {
                        if let Some(alias_cmd) = cmds.get(alias) {
                            let stdout = Logger::root(STDOUT_SW.drain().fuse(), o!());
                            let a_cmd = alias_cmd.command().clone();
                            trace!(stdout, "multiplex"; "hostname" => t_hostname, "alias" => a_cmd);
                            a_cmd
                        } else {
                            cmd.clone()
                        }
                    } else {
                        cmd.clone()
                    }
                } else {
                    cmd.clone()
                }
            } else {
                cmd.clone()
            }
        } else {
            cmd.clone()
        };

        children.push(thread::spawn(move || {
            if let Err(e) = execute(t_hostname, t_cmd, username, pem, (&hn[..], port)) {
                println!("{}", e.description());
            }
        }));
    }

    let mut errors = Vec::new();
    for child in children {
        if let Err(e) = child.join() {
            println!("{:?}", e);
            errors.push(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        // TODO: Fix this error
        Err(MusshErr::Unknown)
    }
}

fn setup_file_log(matches: &ArgMatches, level: Level, stdout: bool) {
    let mut file_drain = None;
    let mut file_path = if let Some(logdir) = matches.value_of("logdir") {
        PathBuf::from(logdir)
    } else if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(DOT_DIR);
        home_dir
    } else {
        PathBuf::new()
    };

    if stdout {
        file_path.push(STDOUT_FILE);
    } else {
        file_path.push(STDERR_FILE);
    }

    if let Ok(log_file) = OpenOptions::new().create(true).append(true).open(file_path) {
        let fmt = Format::new(FormatMode::Full, ColorDecorator::new_plain());
        file_drain = Some(async_stream(log_file, fmt));
    }

    let base = if stdout {
        level_filter(level, slog_term::streamer().async().full().build())
    } else {
        level_filter(level, slog_term::streamer().stderr().async().full().build())
    };

    if let Some(file) = file_drain {
        if stdout {
            STDOUT_SW.set(duplicate(base, file)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Unable to duplicate drain")));
        } else {
            STDERR_SW.set(duplicate(base, file)
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "Unable to duplicate drain")));
        }
    } else if stdout {
        STDOUT_SW.set(base);
    } else {
        STDERR_SW.set(base);
    }
}

pub fn run(opt_args: Option<Vec<&str>>) -> i32 {
    let app = App::new("mussh")
        .version(crate_version!())
        .author("Jason Ozias <jason.g.ozias@gmail.com>")
        .about("ssh multiplexing client")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("CONFIG")
            .help("Specify a non-standard path for the config file.")
            .takes_value(true))
        .arg(Arg::with_name("logdir")
            .short("l")
            .long("logdir")
            .value_name("LOGDIR")
            .help("Specify a non-standard path for the log files.")
            .takes_value(true))
        .arg(Arg::with_name("dry_run")
            .long("dryrun")
            .help("Parse config and setup the client, but don't run it."))
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Set the output verbosity level (more v's = more verbose)"))
        .arg(Arg::with_name("hosts")
            .value_name("hosts")
            .help("The hosts to multiplex the command over")
            .index(1)
            .required(true))
        .arg(Arg::with_name("command")
            .value_name("CMD")
            .help("The command to multiplex")
            .index(2)
            .required(true));

    let matches = if let Some(args) = opt_args {
        app.get_matches_from(args)
    } else {
        app.get_matches()
    };

    // Setup the logging
    let level = match matches.occurrences_of("verbose") {
        0 => Level::Info,
        1 => Level::Debug,
        2 | _ => Level::Trace,
    };

    // Create the dot dir if it doesn't exist.
    if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(DOT_DIR);
        if let Err(_) = fs::metadata(&home_dir) {
            if let Err(_) = fs::create_dir_all(home_dir) {
                return 1;
            }
        }
    }

    setup_file_log(&matches, level, true);
    setup_file_log(&matches, level, false);

    if matches.is_present("dry_run") {
        let stdout = Logger::root(STDOUT_SW.drain().fuse(), o!());
        warn!(stdout, "run"; "message" => "Not starting multiplex!", "dryrun" => "true");
        0
    } else if let Err(e) = multiplex(MusshToml::new(&matches), matches) {
        let stderr = Logger::root(STDERR_SW.drain().fuse(), o!());
        error!(stderr, "run"; "error" => "error running multiplex", "detail" => format!("{}", e));
        1
    } else {
        0
    }
}

use {MusshResult, STDERR_SW, STDOUT_SW};
use clap::{App, Arg, ArgMatches};
use config::MusshToml;
use error::MusshErr;
use slog::Level;
use slog::drain::{self, IntoLogger};
use slog_json;
use slog_term;
use ssh2::Session;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::{Command, Stdio};
use std::thread;

fn setup_hostnames(config: &MusshToml, matches: &ArgMatches) -> MusshResult<Vec<String>> {
    let stdout = STDOUT_SW.drain().into_logger(o!());
    let mut hostnames = Vec::new();
    if let Some(hosts_arg) = matches.value_of("hosts") {
        if let Some(hosts) = config.hostlist() {
            for (name, host_config) in hosts {
                if name == hosts_arg {
                    hostnames = host_config.hostnames().clone();
                    trace!(stdout, "multiplex", "hostnames" => format!("{:?}", hostnames));
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
    let stdout = STDOUT_SW.drain().into_logger(o!());
    let mut cmd = String::new();
    if let Some(cmd_arg) = matches.value_of("command") {
        if let Some(cmds) = config.cmd() {
            for (name, command) in cmds {
                if name == cmd_arg {
                    cmd = command.command().clone();
                    trace!(stdout, "multiplex", "command" => cmd);
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

fn setup_host(config: &MusshToml, hostname: &str) -> MusshResult<(String, u16)> {
    if let Some(hosts) = config.hosts() {
        if let Some(host) = hosts.get(hostname) {
            let hn = host.hostname();
            let port = host.port().unwrap_or(22);
            Ok((hn.clone(), port))
        } else {
            // TODO: fix this error
            Err(MusshErr::Unknown)
        }
    } else {
        // TODO: fix this error
        Err(MusshErr::Unknown)
    }
}

fn execute<A: ToSocketAddrs>(hostname: String, command: String, host: A) -> MusshResult<()> {
    let stdout = STDOUT_SW.drain().into_logger(o!());
    if &hostname == "localhost" {
        let mut cmd_iter = command.split_whitespace();
        if let Some(cmd) = cmd_iter.next() {
            let args: Vec<&str> = cmd_iter.collect();
            let mut cmd = Command::new(cmd);
            cmd.args(&args);
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            if let Ok(mut child) = cmd.spawn() {
                let stdout_reader = BufReader::new(child.stdout.take().expect(""));
                let stderr_reader = BufReader::new(child.stderr.take().expect(""));
                let blah = stdout.clone();
                let hn = hostname.clone();
                let stdout_child = thread::spawn(move || {
                    for line in stdout_reader.lines() {
                        trace!(blah, "execute", "hostname" => hn, "line" => line.expect(""));
                    }
                });

                let blah1 = stdout.clone();
                let hn1 = hostname.clone();
                let stderr_child = thread::spawn(move || {
                    for line in stderr_reader.lines() {
                        trace!(blah1, "execute", "hostname" => hn1, "line" => line.expect(""));
                    }
                });

                let _ = stdout_child.join();
                let _ = stderr_child.join();
                child.wait().expect("command wasn't running");
            }
        }
        Ok(())
    } else {
        if let Some(mut sess) = Session::new() {
            trace!(stdout, "execute", "message" => "Session established");
            let tcp = TcpStream::connect(host)?;
            sess.handshake(&tcp)?;
            sess.userauth_agent("jozias")?;

            if sess.authenticated() {
                let mut channel = sess.channel_session()?;
                channel.exec(&command)?;
                let reader = BufReader::new(channel);
                for line in reader.lines() {
                    trace!(stdout, "execute", "hostname" => hostname, "line" => line.expect(""));
                }
            } else {
                return Err(MusshErr::Auth);
            }
        } else {
            return Err(MusshErr::InvalidSshSession);
        }
        Ok(())
    }
}

fn multiplex(config: MusshToml, matches: ArgMatches) -> MusshResult<()> {
    let hostnames = setup_hostnames(&config, &matches)?;
    let cmd = setup_command(&config, &matches)?;
    let mut children = Vec::new();

    for hostname in hostnames.into_iter() {
        let t_hostname = hostname.clone();
        let t_cmd = cmd.clone();
        let (hn, port) = setup_host(&config, &t_hostname)?;
        children.push(thread::spawn(move || execute(t_hostname, t_cmd, (&hn[..], port))));
    }

    let mut errors = Vec::new();
    for child in children {
        if let Err(e) = child.join() {
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
        .arg(Arg::with_name("dry_run")
            .long("dryrun")
            .help("Parse config and setup the client, but don't run it."))
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Set the output verbosity level (more v's = more verbose)"))
        .arg(Arg::with_name("json")
            .short("j")
            .long("json")
            .help("Enable json logging at the given path")
            .value_name("PATH")
            .takes_value(true))
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
        0 => Level::Error,
        1 => Level::Warning,
        2 => Level::Info,
        3 => Level::Debug,
        4 | _ => Level::Trace,
    };

    let mut stdout_json_drain = None;
    if let Some(json_path) = matches.value_of("json") {
        if let Ok(json_file) = OpenOptions::new().create(true).append(true).open(json_path) {
            stdout_json_drain = Some(drain::stream(json_file, slog_json::new()));
        }
    }

    let stdout_base = drain::async_stream(io::stdout(), slog_term::format_colored());

    if let Some(json) = stdout_json_drain {
        STDOUT_SW.set(drain::filter_level(level, drain::duplicate(stdout_base, json)));
    } else {
        STDOUT_SW.set(drain::filter_level(level, stdout_base));
    }

    let mut stderr_json_drain = None;
    if let Some(json_path) = matches.value_of("json") {
        if let Ok(json_file) = OpenOptions::new().create(true).append(true).open(json_path) {
            stderr_json_drain = Some(drain::stream(json_file, slog_json::new()));
        }
    }

    let stderr_base = drain::async_stream(io::stderr(), slog_term::format_colored());

    if let Some(json) = stderr_json_drain {
        STDERR_SW.set(drain::filter_level(level, drain::duplicate(stderr_base, json)));
    } else {
        STDERR_SW.set(drain::filter_level(level, stderr_base));
    }

    if matches.is_present("dry_run") {
        let stdout = STDOUT_SW.drain().into_logger(o!());
        warn!(stdout, "run", "message" => "Not starting multiplex!", "dryrun" => "true");
        0
    } else if let Err(e) = multiplex(MusshToml::new(&matches), matches) {
        let stderr = STDERR_SW.drain().into_logger(o!());
        error!(stderr, "run", "error" => "error running multiplex", "detail" => format!("{}", e));
        1
    } else {
        0
    }
}

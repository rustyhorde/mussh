// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `mussh` run sub-command.
use clap::ArgMatches;
use config::{self, Config, FileDrain, MusshToml};
use error::{ErrorKind, Result};
use slog::{Drain, Logger};
use slog_async;
use ssh2::Session;
use std::collections::BTreeMap;
use std::{env, fs, thread};
use std::io::{BufRead, BufReader};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Instant;

/// Setup the hostnames from the toml config.
fn setup_hostnames(config: &Config) -> Result<Vec<String>> {
    let stdout = config.stdout();
    let mut hostnames = Vec::new();
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let hosts = toml.hostlist();
    let mut tmp_hns = Vec::new();
    for host in &config.hosts() {
        if let Some(hn) = hosts.get(&host.to_string()) {
            tmp_hns.extend(hn.hostnames().to_vec());
        }
    }

    for host in &config.hosts() {
        if host.starts_with('!') {
            let (_, hn) = (*host).split_at(1);
            warn!(stdout, "setup_hostnames"; "removing host" => hn);
            tmp_hns = tmp_hns.iter().filter(|x| *x != hn).cloned().collect();
        }
    }

    hostnames.extend(tmp_hns);

    for hostname in &hostnames {
        trace!(stdout, "setup_hostnames";  "including host" => hostname);
    }

    if hostnames.is_empty() {
        Err(ErrorKind::NoValidHosts.into())
    } else {
        Ok(hostnames)
    }
}

/// Setup a command from the toml config.
fn setup_command(config: &Config) -> Result<String> {
    let stdout = config.stdout();
    let mut cmd = String::new();
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let cmds = toml.cmd();
    for (name, command) in cmds {
        if name == config.cmd() {
            cmd.push_str(command.command());
            trace!(stdout, "setup_command"; "base command" => &cmd);
            break;
        }
    }

    if cmd.is_empty() {
        Err(ErrorKind::InvalidCmd.into())
    } else {
        Ok(cmd)
    }
}

/// Host Configuration tuple.
type ConfigTuple = (String, String, u16, Option<String>, Option<BTreeMap<String, String>>);

/// Setup a host given a hostname.
fn setup_host(config: &Config, hostname: &str) -> Result<ConfigTuple> {
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let hosts = toml.hosts();
    let host = hosts
        .get(hostname)
        .ok_or_else(|| ErrorKind::HostNotConfigured(hostname.to_string()))?;
    let username = host.username();
    let hn = host.hostname();
    let pem = if let Some(pem) = host.pem() {
        Some(pem.to_string())
    } else {
        None
    };
    let port = host.port().unwrap_or(22);
    let alias = if let Some(al) = host.alias() {
        Some(al.clone())
    } else {
        None
    };
    Ok((username.to_string(), hn.to_string(), port, pem, alias))
}

/// Setup the command aliases.
fn setup_alias(config: &Config, alias: Option<BTreeMap<String, String>>) -> Result<String> {
    let alias_map = alias.ok_or_else(|| ErrorKind::InvalidToml)?;
    let alias_name = alias_map
        .get(config.cmd())
        .ok_or_else(|| ErrorKind::InvalidToml)?;
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let cmds = toml.cmd();
    let alias_cmd = cmds.get(alias_name).ok_or_else(|| ErrorKind::InvalidToml)?;
    Ok(alias_cmd.command().to_string())
}

/// Execute the command.
fn execute(logs: (&Logger, &Logger),
           host: &str,
           hostname: &str,
           port: u16,
           command: &str,
           username: &str,
           pem: Option<String>)
           -> Result<()> {
    let (stdout, stderr) = logs;
    let mut host_file_path = if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(config::DOT_DIR);
        home_dir
    } else {
        PathBuf::new()
    };

    host_file_path.push(host);
    host_file_path.set_extension("log");

    let file_drain = FileDrain::new(host_file_path)?;
    let async_file_drain = slog_async::Async::new(file_drain).build().fuse();
    // let level_file_drain = LevelFilter::new(async_file_drain, Level::Error).fuse();
    let file_logger = Logger::root(async_file_drain, o!());
    let timer = Instant::now();

    if host == "lh" {
        let mut cmd = Command::new("/usr/bin/fish");
        cmd.arg("-c");
        cmd.arg(command);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        if let Ok(mut child) = cmd.spawn() {
            let stdout_reader = BufReader::new(child.stdout.take().expect(""));
            for line in stdout_reader.lines() {
                if let Ok(line) = line {
                    trace!(file_logger, "{}", line);
                }
            }

            let status = child.wait()?;
            if status.success() {
                info!(
                    stdout,
                    "execute";
                    "host" => host,
                    "duration" => timer.elapsed().as_secs()
                );
            } else {
                error!(
                    stderr,
                    "execute";
                    "host" => host,
                    "duration" => timer.elapsed().as_secs()
                );
            }
        }
    } else if let Some(mut sess) = Session::new() {
        let host_tuple = (hostname, port);
        let tcp = TcpStream::connect(host_tuple)?;
        sess.handshake(&tcp)?;
        if let Some(pem) = pem {
            sess.userauth_pubkey_file(username, None, Path::new(&pem), None)?;
        } else {
            trace!(stdout, "execute"; "message" => "Agent Auth Setup", "username" => username);
            let mut agent = sess.agent()?;
            agent.connect()?;
            agent.list_identities()?;
            for identity in agent.identities() {
                if let Ok(ref id) = identity {
                    if agent.userauth(username, id).is_ok() {
                        break;
                    }
                }
            }
            agent.disconnect()?;
        }

        if sess.authenticated() {
            trace!(stdout, "execute"; "message" => "Authenticated");
            let mut channel = sess.channel_session()?;
            channel.exec(command)?;

            {
                let stdout_stream = channel.stream(0);
                let stdout_reader = BufReader::new(stdout_stream);

                for line in stdout_reader.lines() {
                    if let Ok(line) = line {
                        trace!(file_logger, "{}", line);
                    }
                }
            }

            match channel.exit_status() {
                Ok(code) => {
                    if code == 0 {
                        info!(
                            stdout,
                            "execute";
                            "host" => host,
                            "duration" => timer.elapsed().as_secs()
                        );
                    } else {
                        error!(
                            stderr,
                            "execute";
                            "host" => host,
                            "duration" => timer.elapsed().as_secs()
                        );
                    }
                }
                Err(e) => {
                    error!(
                        stderr,
                        "execute"; "hostname" => hostname, "error" => format!("{}", e)
                    );
                }
            }
        } else {
            return Err(ErrorKind::SshAuthentication.into());
        }
    } else {
        return Err(ErrorKind::SshSession.into());
    }

    Ok(())
}

/// Run the commond over the hosts.
fn multiplex(config: &Config) -> Result<()> {
    let hostnames = setup_hostnames(config)?;
    let count = hostnames.len();
    let base_cmd = setup_command(config)?;
    let (tx, rx) = mpsc::channel();
    let mut errors = Vec::new();

    for host in hostnames {
        let stdout = config.stdout();
        let stderr = config.stderr();
        let (username, hostname, port, pem, alias) = setup_host(config, &host)?;
        let cmd = match setup_alias(config, alias) {
            Ok(alias_cmd) => alias_cmd,
            Err(_) => base_cmd.clone(),
        };
        trace!(stdout, "multiplex"; "hostname" => &host, "cmd" => &cmd);
        let h_tx = tx.clone();
        thread::spawn(move || {
            h_tx.send(execute((&stdout, &stderr),
                              &host,
                              &hostname,
                              port,
                              &cmd,
                              &username,
                              pem))
                .expect("badness");
        });

        if config.sync() {
            match rx.recv() {
                Ok(_) => {}
                Err(e) => errors.push(e),
            }
        }
    }

    if !config.sync() {
        for _ in 0..count {
            match rx.recv() {
                Ok(_) => {}
                Err(e) => errors.push(e),
            }
        }
    }
    Ok(())
}

/// Run the `run` sub-command.
pub fn cmd(config: &mut Config,
           sub_m: &ArgMatches,
           stdout: &Logger,
           stderr: &Logger)
           -> Result<i32> {
    if let Some(cmd) = sub_m.value_of("command") {
        config.set_cmd(cmd);
    }

    if sub_m.is_present("sync") {
        config.set_sync(true);
    }

    if let Some(hosts_iter) = sub_m.values_of("hosts") {
        let hosts: Vec<&str> = hosts_iter.collect();
        for host in &hosts {
            trace!(stdout, "{}", host);
        }
        config.set_hosts(hosts);
    }

    // Create the dot dir if it doesn't exist.
    if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(config::DOT_DIR);
        if fs::metadata(&home_dir).is_err() || fs::create_dir_all(home_dir).is_err() {
            error!(stderr, "cannot use/create the home directory!");
            return Ok(1);
        }
    }

    // Parse the toml and add to config if successful.
    let final_config = match MusshToml::new(config) {
        Ok(toml) => config.set_toml(toml),
        Err(e) => {
            error!(stderr, "{}", e);
            return Err(e);
        }
    };

    if sub_m.is_present("dry_run") {
        Ok(0)
    } else {
        match multiplex(final_config) {
            Ok(_) => Ok(0),
            Err(e) => {
                error!(stderr, "{}", e);
                Err(e)
            }
        }
    }
}

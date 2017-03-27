// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! runtime for `mussh`.
use clap::{App, Arg};
use config::{self, Config, FileDrain, MusshToml};
use error::{ErrorKind, Result};
use slog::{Drain, Level, LevelFilter, Logger};
use slog_async;
use std::{env, fs, thread};
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Instant;

/// Setup the hostnames from the toml config.
fn setup_hostnames(config: &Config) -> Result<Vec<String>> {
    let stdout = config.stdout();
    let mut hostnames = Vec::new();
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let hosts = toml.hostlist().ok_or_else(|| ErrorKind::InvalidHostList)?;
    let mut tmp_hns = Vec::new();
    for host in &config.hosts() {
        if let Some(hn) = hosts.get(&host.to_string()) {
            tmp_hns.extend(hn.hostnames().to_vec());
        }
    }

    for host in &config.hosts() {
        if host.starts_with('!') {
            let (_, hn) = (*host).split_at(1);
            warn!(stdout, "setupt_hostnames"; "removing host" => hn);
            tmp_hns = tmp_hns.iter()
                .filter(|x| *x != hn)
                .cloned()
                .collect();
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
    let cmds = toml.cmd().ok_or_else(|| ErrorKind::InvalidCmd)?;
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

type ConfigTuple = (String, String, u16, Option<String>, Option<HashMap<String, String>>);

fn setup_host(config: &Config, hostname: &str) -> Result<ConfigTuple> {
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let hosts = toml.hosts().ok_or_else(|| ErrorKind::InvalidHosts)?;
    let host = hosts.get(hostname)
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

fn setup_alias(config: &Config, alias: Option<HashMap<String, String>>) -> Result<String> {
    let alias_map = alias.ok_or_else(|| ErrorKind::InvalidToml)?;
    let alias = alias_map.get(config.cmd()).ok_or_else(|| ErrorKind::InvalidToml)?;
    let toml = config.toml().ok_or_else(|| ErrorKind::InvalidToml)?;
    let cmds = toml.cmd().ok_or_else(|| ErrorKind::InvalidToml)?;
    let alias_cmd = cmds.get(alias).ok_or_else(|| ErrorKind::InvalidToml)?;
    Ok(alias_cmd.command().to_string())
}

fn execute(hostname: &str,
           _port: u16,
           _command: &str,
           _username: &str,
           _pem: &Option<String>)
           -> Result<()> {
    let mut host_file_path = if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(config::DOT_DIR);
        home_dir
    } else {
        PathBuf::new()
    };

    host_file_path.push(hostname);
    host_file_path.set_extension("log");

    let file_drain = FileDrain::new(host_file_path)?;
    let async_file_drain = slog_async::Async::new(file_drain).build().fuse();
    let level_file_drain = LevelFilter::new(async_file_drain, Level::Error).fuse();
    let _file_logger = Logger::root(level_file_drain, o!());
    let _timer = Instant::now();
    Ok(())
}

/// Run the commond over the hosts.
fn multiplex(config: &Config) -> Result<()> {
    let hostnames = setup_hostnames(config)?;
    let count = hostnames.len();
    let base_cmd = setup_command(config)?;
    let (tx, rx) = mpsc::channel();

    for host in hostnames {
        let stdout = config.stdout();
        let (username, hostname, port, pem, alias) = setup_host(config, &host)?;
        let cmd = match setup_alias(config, alias) {
            Ok(alias_cmd) => alias_cmd,
            Err(_) => base_cmd.clone(),
        };
        trace!(stdout, "multiplex"; "hostname" => host, "cmd" => &cmd);
        let h_tx = tx.clone();
        thread::spawn(move || {
            h_tx.send(execute(&hostname, port, &cmd, &username, &pem)).expect("badness");
        });
    }

    let mut errors = Vec::new();
    for _ in 0..count {
        match rx.recv() {
            Ok(_) => {}
            Err(e) => errors.push(e),
        }
    }
    Ok(())
}

/// Run `mussh`
pub fn run() -> Result<i32> {
    let mut config: Config = Default::default();
    let stdout = config.stdout();
    let stderr = config.stderr();

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Jason Ozias <jason.g.ozias@gmail.com>")
        .about("ssh multiplexing client")
        .arg(Arg::with_name("config")
                 .short("c")
                 .long("config")
                 .value_name("CONFIG")
                 .help("Specify a non-standard path for the TOML config file.")
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
        .arg(Arg::with_name("command")
                 .value_name("CMD")
                 .help("The command to multiplex")
                 .index(1)
                 .required(true))
        .arg(Arg::with_name("hosts")
                 .value_name("HOSTS")
                 .multiple(true)
                 .help("The hosts to multiplex the command over")
                 .index(2)
                 .required(true))
        .get_matches();

    // Setup the logging
    let level = match matches.occurrences_of("verbose") {
        0 => Level::Info,
        1 => Level::Debug,
        2 | _ => Level::Trace,
    };
    config.set_stdout_level(level);

    if let Some(toml_dir_string) = matches.value_of("config") {
        config.set_toml_dir(toml_dir_string);
    }

    if let Some(log_dir_string) = matches.value_of("logdir") {
        config.set_log_dir(log_dir_string);
    }

    if let Some(cmd) = matches.value_of("command") {
        config.set_cmd(cmd);
    }

    if let Some(hosts_iter) = matches.values_of("hosts") {
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
    let toml_dir = config.toml_dir();
    config.set_toml(MusshToml::new(toml_dir)?);

    if matches.is_present("dry_run") {
        Ok(0)
    } else {
        match multiplex(&config) {
            Ok(_) => Ok(0),
            Err(e) => {
                error!(stderr, "{}", e);
                Err(e)
            }
        }
    }
}

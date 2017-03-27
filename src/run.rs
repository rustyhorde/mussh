// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! runtime for `mussh`.
use clap::{App, Arg, ArgMatches};
use config::{self, Logging, MusshToml};
use error::{ErrorKind, Result};
use slog::Level;
use std::{env, fs};
use std::io::{self, Write};

/// Setup the hostnames from the toml config.
fn setup_hostnames<'a>(config: &'a MusshToml,
                       logging: &'a Logging,
                       matches: &'a ArgMatches)
                       -> Result<Vec<String>> {
    let stdout = logging.stdout();
    let mut hostnames = Vec::new();
    if let Some(hosts_arg) = matches.value_of("hosts") {
        if let Some(hosts) = config.hostlist() {
            for (name, host_config) in hosts {
                if name == hosts_arg {
                    let hosts: Vec<String> = host_config.hostnames()
                        .iter()
                        .map(|x| x.clone())
                        .collect();
                    hostnames.extend(hosts);
                    for hostname in &hostnames {
                        trace!(stdout, "setup_hostnames";  "{}" => hostname);
                    }
                    break;
                }
            }
        }
    } else {
        return Err(ErrorKind::InvalidHosts.into());
    }

    if hostnames.is_empty() {
        Err(ErrorKind::InvalidHosts.into())
    } else {
        Ok(hostnames)
    }
}

/// Setup a command from the toml config.
fn setup_command(config: &MusshToml, logging: &Logging, matches: &ArgMatches) -> Result<String> {
    let stdout = logging.stdout();
    let mut cmd = String::new();
    if let Some(cmd_arg) = matches.value_of("command") {
        if let Some(cmds) = config.cmd() {
            for (name, command) in cmds {
                if name == cmd_arg {
                    cmd.push_str(command.command());
                    trace!(stdout, "setup_command"; "command" => &cmd);
                    break;
                }
            }
        }
    } else {
        return Err(ErrorKind::InvalidCmd("command not specified as arg".to_string()).into());
    }

    if cmd.is_empty() {
        Err(ErrorKind::InvalidCmd("empty command".to_string()).into())
    } else {
        Ok(cmd)
    }
}

/// Run the commond over the hosts.
fn multiplex(config: &MusshToml, logging: &Logging, matches: &ArgMatches) -> Result<()> {
    let _hostnames = setup_hostnames(config, logging, matches)?;
    let _cmd = setup_command(config, logging, matches)?;
    Ok(())
}

/// Run `mussh`
pub fn run() -> Result<i32> {
    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
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
        .arg(Arg::with_name("command")
                 .value_name("CMD")
                 .help("The command to multiplex")
                 .index(1)
                 .required(true))
        .arg(Arg::with_name("hosts")
                 .value_name("hosts")
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

    // Create the dot dir if it doesn't exist.
    if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(config::DOT_DIR);
        if fs::metadata(&home_dir).is_err() || fs::create_dir_all(home_dir).is_err() {
            writeln!(io::stderr(), "home dir is bad").expect("badness");
            return Ok(1);
        }
    }

    let config = MusshToml::new(&matches)?;
    let mut logging: Logging = Default::default();
    logging.set_stdout_level(level);

    if matches.is_present("dry_run") {
        return Ok(0);
    } else {
        // placeholder
        match multiplex(&config, &logging, &matches) {
            Ok(_) => Ok(0),
            Err(e) => Err(e),
        }
    }
}

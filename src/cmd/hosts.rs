// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `host` sub-command.
use clap::ArgMatches;
use config::{Config, Host, MusshToml};
use error::{ErrorKind, Result};
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::str::FromStr;
use term;
use toml;
use util;

/// Run the `host-list` sub-command.
pub fn list_cmd(config: &mut Config) -> Result<i32> {
    // Parse the toml.
    let toml_dir = config.toml_dir();
    match MusshToml::new(toml_dir) {
        Ok(toml) => {
            let mut t = term::stdout().ok_or_else(|| ErrorKind::NoTerm)?;

            // List out the hosts
            if let Some(hosts) = toml.hosts() {
                let mut max_k = 0;
                for k in hosts.keys() {
                    let len_k = k.len();
                    if len_k > max_k {
                        max_k = len_k;
                    }
                }

                // For sorting
                let mut bmap = BTreeMap::new();
                for (k, v) in hosts {
                    bmap.insert(k, v);
                }

                for (k, v) in bmap {
                    t.fg(term::color::GREEN)?;
                    t.attr(term::Attr::Bold)?;
                    write!(t, "  {}", util::pad_left(&k, max_k))?;
                    t.reset()?;
                    writeln!(t, "  {}", v)?;
                    t.flush()?;
                }
            }
        }
        Err(e) => {
            error!(config.stderr(), "{}", e);
            return Err(e);
        }
    }

    Ok(0)
}

/// Run the `hosts-add` sub-command.
pub fn add_cmd(config: &mut Config, add_m: &ArgMatches) -> Result<i32> {
    let mut host: Host = Default::default();

    if let Some(username) = add_m.value_of("username") {
        host.set_username(username);
    }

    if let Some(hostname) = add_m.value_of("hostname") {
        host.set_hostname(hostname);
    }

    if let Some(port) = add_m.value_of("port") {
        let p = u16::from_str(port)?;
        host.set_port(p);
    }

    // Parse the toml.
    let mut toml = match MusshToml::new(config.toml_dir()) {
        Ok(toml) => toml,
        Err(_) => Default::default(),
    };

    let mut toml_file = OpenOptions::new().create(true)
        .write(true)
        .open("/home/jozias/projects/mussh/mussh.new.toml")?;

    toml.add_host("test", host);
    toml_file.write_all(&toml::to_vec(&toml)?)?;

    Ok(0)
}

/// Run the `host` sub-command.
pub fn cmd(config: &mut Config, sub_m: &ArgMatches) -> Result<i32> {
    match sub_m.subcommand() {
        // 'host-list' subcommand
        ("list", Some(_)) => list_cmd(config),
        ("add", Some(add_m)) => add_cmd(config, add_m),
        _ => Err(ErrorKind::SubCommand.into()),
    }
}

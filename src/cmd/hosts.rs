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
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::str::FromStr;
use term;
use toml;
use util;

/// Run the `host-list` sub-command.
pub fn list_cmd(config: &mut Config) -> Result<i32> {
    // Parse the toml.
    match MusshToml::new(config) {
        Ok(toml) => {
            let mut t = term::stdout().ok_or_else(|| ErrorKind::NoTerm)?;

            // List out the hosts
            let hosts = toml.hosts();
            let mut max_k = 0;
            for k in hosts.keys() {
                let len_k = k.len();
                if len_k > max_k {
                    max_k = len_k;
                }
            }

            for (k, v) in hosts {
                t.fg(term::color::GREEN)?;
                t.attr(term::Attr::Bold)?;
                write!(t, "  {}", util::pad_left(&k, max_k))?;
                t.reset()?;
                writeln!(t, "  {}", v)?;
                t.flush()?;
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

    if let Some(pem) = add_m.value_of("pem") {
        host.set_pem(pem);
    }

    // Parse the toml.
    let mut toml = match MusshToml::new(config) {
        Ok(toml) => toml,
        Err(_) => Default::default(),
    };

    if let Some(ref pb) = config.toml_dir() {
        let mut bk_p = pb.clone();
        bk_p.pop();
        bk_p.push("mussh.toml.bk");
        fs::copy(pb, bk_p)?;
        let mut toml_file = OpenOptions::new().create(true)
            .truncate(true)
            .write(true)
            .open(pb)?;

        toml.add_host("test", host);
        toml_file.write_all(&toml::to_vec(&toml)?)?;
        Ok(0)
    } else {
        error!(config.stderr(), "Unable to determine TOML file path!");
        Err(ErrorKind::Config.into())
    }
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

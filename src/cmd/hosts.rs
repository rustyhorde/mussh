// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `host` sub-command.
use clap::ArgMatches;
use cmd;
use config::{Config, Host, MusshToml};
use error::{ErrorKind, Result};
use std::io::Write;
use std::str::FromStr;
use term;
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
                write!(t, "  {}", util::pad_left(k, max_k))?;
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
pub fn add_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let mut host: Host = Default::default();

        if let Some(username) = matches.value_of("username") {
            host.set_username(username);
        }

        if let Some(hostname) = matches.value_of("hostname") {
            host.set_hostname(hostname);
        }

        if let Some(port) = matches.value_of("port") {
            let p = u16::from_str(port)?;
            host.set_port(p);
        }

        if let Some(pem) = matches.value_of("pem") {
            host.set_pem(pem);
        }

        let mut toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        toml.add_host(name, host);

        match cmd::write_toml(config, &toml) {
            Ok(i) => {
                info!(config.stdout(), "'{}' added successfully", name);
                Ok(i)
            }
            Err(e) => Err(e),
        }
    } else {
        Err(ErrorKind::SubCommand.into())
    }
}

/// Run the `hosts-remove` sub-command.
pub fn remove_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let mut toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        let mut hosts = toml.hosts().clone();
        let rem = hosts.remove(name);

        if rem.is_none() {
            Err(ErrorKind::NoValidHosts.into())
        } else {
            toml.set_hosts(hosts);
            match cmd::write_toml(config, &toml) {
                Ok(i) => {
                    info!(config.stdout(), "'{}' removed successfully", name);
                    Ok(i)
                }
                Err(e) => Err(e),
            }
        }
    } else {
        Err(ErrorKind::SubCommand.into())
    }
}

/// Run the `hosts-update` sub-command.
pub fn update_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        let hosts = toml.hosts();

        if let Some(h) = hosts.get(name) {
            let mut host = h.clone();
            let mut mut_toml = toml.clone();
            if let Some(username) = matches.value_of("username") {
                host.set_username(username);
            }

            if let Some(hostname) = matches.value_of("hostname") {
                host.set_hostname(hostname);
            }

            if let Some(port) = matches.value_of("port") {
                let p = u16::from_str(port)?;
                host.set_port(p);
            }

            if let Some(pem) = matches.value_of("pem") {
                host.set_pem(pem);
            }

            mut_toml.add_host(name, host);

            match cmd::write_toml(config, &mut_toml) {
                Ok(i) => {
                    info!(config.stdout(), "'{}' updated successfully", name);
                    Ok(i)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(ErrorKind::HostDoesNotExist.into())
        }
    } else {
        Err(ErrorKind::SubCommand.into())
    }
}

/// Run the `hosts` sub-command.
pub fn cmd(config: &mut Config, sub_m: &ArgMatches) -> Result<i32> {
    match sub_m.subcommand() {
        // 'hosts-list' subcommand
        ("list", Some(_)) => list_cmd(config),
        // 'hosts-add' subcommand
        ("add", Some(matches)) => add_cmd(config, matches),
        // 'hosts-remove' subcommand
        ("remove", Some(matches)) => remove_cmd(config, matches),
        // 'hosts-update' subcommand
        ("update", Some(matches)) => update_cmd(config, matches),
        _ => Err(ErrorKind::SubCommand.into()),
    }
}

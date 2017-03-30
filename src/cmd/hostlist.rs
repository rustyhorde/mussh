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
use config::{Config, Hosts, MusshToml};
use error::{ErrorKind, Result};
use slog::Logger;
use std::io::Write;
use term;
use util;

/// Run the `host-list` sub-command.
pub fn list_cmd(config: &mut Config, stderr: &Logger) -> Result<i32> {
    // Parse the toml.
    match MusshToml::new(config) {
        Ok(toml) => {
            let mut t = term::stdout().ok_or_else(|| ErrorKind::NoTerm)?;

            // List out the hostlists
            let hostlist = toml.hostlist();
            let mut max_k = 0;
            for k in hostlist.keys() {
                let len_k = k.len();
                if len_k > max_k {
                    max_k = len_k;
                }
            }

            for (k, v) in hostlist {
                t.fg(term::color::GREEN)?;
                t.attr(term::Attr::Bold)?;
                write!(t, "  {}", util::pad_left(&k, max_k))?;
                t.reset()?;
                writeln!(t, "  {}", v)?;
                t.flush()?;
            }
        }
        Err(e) => {
            error!(stderr, "{}", e);
            return Err(e);
        }
    }

    Ok(0)
}

/// Run the `hostlist-add` sub-command.
pub fn add_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let mut hosts: Hosts = Default::default();

        if let Some(host_iter) = matches.values_of("hosts") {
            let h_vec = host_iter.map(|x| x.to_string()).collect();
            hosts.set_hostnames(h_vec);
        }

        let mut toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        toml.add_hostlist(name, hosts);

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

/// Run the `hostlist-remove` sub-command.
pub fn remove_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let mut toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        let mut hostlist = toml.hostlist().clone();
        let rem = hostlist.remove(name);

        if rem.is_none() {
            Err(ErrorKind::NoValidHostlist.into())
        } else {
            toml.set_hostlist(hostlist);
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

/// Run the `hostlist-update` sub-command.
pub fn update_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        let hostlist = toml.hostlist();

        if let Some(hl) = hostlist.get(name) {
            let mut hostlist = hl.clone();
            let mut mut_toml = toml.clone();
            if let Some(hosts_iter) = matches.values_of("hosts") {
                let h_vec = hosts_iter.map(|x| x.to_string()).collect();
                hostlist.set_hostnames(h_vec);
            }

            mut_toml.add_hostlist(name, hostlist);

            match cmd::write_toml(config, &mut_toml) {
                Ok(i) => {
                    info!(config.stdout(), "'{}' updated successfully", name);
                    Ok(i)
                }
                Err(e) => Err(e),
            }
        } else {
            Err(ErrorKind::HostListDoesNotExist.into())
        }
    } else {
        Err(ErrorKind::SubCommand.into())
    }
}

/// Run the `host` sub-command.
pub fn cmd(config: &mut Config, sub_m: &ArgMatches, stderr: &Logger) -> Result<i32> {
    match sub_m.subcommand() {
        // 'hostlist-list' subcommand
        ("list", Some(_)) => list_cmd(config, stderr),
        // 'hostlist-add' subcommand
        ("add", Some(matches)) => add_cmd(config, matches),
        // 'hostlist-remove' subcommand
        ("remove", Some(matches)) => remove_cmd(config, matches),
        // 'hostlist-update' subcommand
        ("update", Some(matches)) => update_cmd(config, matches),
        _ => Err(ErrorKind::SubCommand.into()),
    }
}

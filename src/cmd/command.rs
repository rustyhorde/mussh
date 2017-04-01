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
use config::{Command, Config, MusshToml};
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
            let cmd = toml.cmd();
            let mut max_k = 0;
            for k in cmd.keys() {
                let len_k = k.len();
                if len_k > max_k {
                    max_k = len_k;
                }
            }

            for (k, v) in cmd {
                t.fg(term::color::GREEN)?;
                t.attr(term::Attr::Bold)?;
                write!(t, "  {}", util::pad_left(&k, max_k))?;
                t.reset()?;
                let sub_cmds = v.command().split(';');

                for (idx, sub_cmd) in sub_cmds.enumerate() {
                    if idx == 0 {
                        writeln!(t, "  {}", sub_cmd)?;
                    } else {
                        let mut padded = String::new();
                        for _ in 0..(max_k + 4) {
                            padded.push(' ');
                        }
                        padded.push_str(sub_cmd.trim());
                        writeln!(t, "{}", padded)?;
                    }
                }
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
        let mut command: Command = Default::default();

        if let Some(cmd) = matches.value_of("cmd") {
            command.set_command(cmd);
        }

        let mut toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        toml.add_cmd(name, command);

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

/// Run the `cmd-remove` sub-command.
pub fn remove_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let mut toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        let mut cmds = toml.cmd().clone();
        let rem = cmds.remove(name);

        if rem.is_none() {
            Err(ErrorKind::NoValidHosts.into())
        } else {
            toml.set_cmd(cmds);
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

/// Run the `cmd-update` sub-command.
pub fn update_cmd(config: &mut Config, matches: &ArgMatches) -> Result<i32> {
    if let Some(name) = matches.value_of("name") {
        let toml = match MusshToml::new(config) {
            Ok(toml) => toml,
            Err(_) => Default::default(),
        };

        let cmds = toml.cmd();

        if let Some(cmd) = cmds.get(name) {
            let mut mut_cmd = cmd.clone();
            let mut mut_toml = toml.clone();
            if let Some(cmd_arg) = matches.value_of("cmd") {
                mut_cmd.set_command(cmd_arg);
            }

            mut_toml.add_cmd(name, mut_cmd);

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

/// Run the `host` sub-command.
pub fn cmd(config: &mut Config, sub_m: &ArgMatches, stderr: &Logger) -> Result<i32> {
    match sub_m.subcommand() {
        // 'cmd-list' subcommand
        ("list", Some(_)) => list_cmd(config, stderr),
        // 'cmd-add' subcommand
        ("add", Some(matches)) => add_cmd(config, matches),
        // 'cmd-remove' subcommand
        ("remove", Some(matches)) => remove_cmd(config, matches),
        // 'cmd-update' subcommand
        ("update", Some(matches)) => update_cmd(config, matches),
        _ => Err(ErrorKind::SubCommand.into()),
    }
}

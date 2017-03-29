// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `host` sub-command.
use clap::ArgMatches;
use config::{self, Config, MusshToml};
use error::{ErrorKind, Result};
use slog::Logger;
use std::{env, fs};
use std::collections::BTreeMap;
use std::io::Write;
use term;
use util;

/// Run the `host-list` sub-command.
pub fn list_cmd(config: &mut Config,
                _sub_m: &ArgMatches,
                _stdout: &Logger,
                stderr: &Logger)
                -> Result<i32> {
    // Create the dot dir if it doesn't exist.
    if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(config::DOT_DIR);
        if fs::metadata(&home_dir).is_err() || fs::create_dir_all(home_dir).is_err() {
            error!(stderr, "cannot use/create the home directory!");
            return Ok(1);
        }
    }

    // Parse the toml.
    let toml_dir = config.toml_dir();
    match MusshToml::new(toml_dir) {
        Ok(toml) => {
            let mut t = term::stdout().ok_or_else(|| ErrorKind::NoTerm)?;

            // List out the hostlists
            if let Some(cmd) = toml.cmd() {
                let mut max = 0;
                for k in cmd.keys() {
                    let len = k.len();
                    if len > max {
                        max = len;
                    }
                }

                // For sorting
                let mut bmap = BTreeMap::new();
                for (k, v) in cmd {
                    bmap.insert(k, v);
                }

                for (k, v) in bmap {
                    t.fg(term::color::GREEN)?;
                    write!(t, "{}", util::pad_left(&k, max))?;
                    t.reset()?;
                    writeln!(t, ": {}", v)?;
                }
            }
        }
        Err(e) => {
            error!(stderr, "{}", e);
            return Err(e);
        }
    }

    Ok(0)
}

/// Run the `host` sub-command.
pub fn cmd(config: &mut Config,
           sub_m: &ArgMatches,
           stdout: &Logger,
           stderr: &Logger)
           -> Result<i32> {
    match sub_m.subcommand() {
        // 'hostlist-list' subcommand
        ("list", Some(sub_m)) => list_cmd(config, sub_m, stdout, stderr),
        _ => Err(ErrorKind::SubCommand.into()),
    }
}

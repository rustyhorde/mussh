// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `host` sub-command.
use clap::ArgMatches;
use config::{Config, MusshToml};
use error::{ErrorKind, Result};
use slog::Logger;
use std::collections::BTreeMap;
use std::io::Write;
use term;
use util;

/// Run the `host-list` sub-command.
pub fn list_cmd(config: &mut Config, stderr: &Logger) -> Result<i32> {
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
            error!(stderr, "{}", e);
            return Err(e);
        }
    }

    Ok(0)
}

/// Run the `host` sub-command.
pub fn cmd(config: &mut Config, sub_m: &ArgMatches, stderr: &Logger) -> Result<i32> {
    match sub_m.subcommand() {
        // 'host-list' subcommand
        ("list", Some(_)) => list_cmd(config, stderr),
        _ => Err(ErrorKind::SubCommand.into()),
    }
}

// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `mussh` sub-commands
use config::{Config, MusshToml};
use error::{ErrorKind, Result};
use std::fs::{self, OpenOptions};
use std::io::Write;
use toml;

pub mod command;
pub mod hostlist;
pub mod hosts;
pub mod run;

/// Write the given TOML out to the `toml_dir` path.
fn write_toml(config: &Config, toml: &MusshToml) -> Result<i32> {
    if let Some(ref pb) = config.toml_dir() {
        let mut bk_p = pb.clone();
        bk_p.pop();
        bk_p.push("mussh.toml.bk");
        fs::copy(pb, bk_p)?;
        let mut toml_file = OpenOptions::new().create(true)
            .truncate(true)
            .write(true)
            .open(pb)?;

        toml_file.write_all(&toml::to_vec(&toml)?)?;
        Ok(0)
    } else {
        error!(config.stderr(), "Unable to determine TOML file path!");
        Err(ErrorKind::Config.into())
    }
}

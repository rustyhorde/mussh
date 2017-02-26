// Copyright (c) 2016 goopy developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! mussh - SSH Multiplexing
#![cfg_attr(feature="cargo-clippy", allow(unseparated_literal_suffix))]
#![deny(missing_docs)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;

extern crate ssh2;
extern crate serde;
extern crate slog_atomic;
extern crate slog_json;
extern crate slog_stream;
extern crate slog_term;
extern crate toml;

use error::MusshErr;
use slog::{Level, level_filter};
use slog_atomic::{AtomicSwitch, AtomicSwitchCtrl};
use std::process;

mod config;
mod error;
mod run;

/// mussh Version
pub const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
/// mussh Package Name
pub const PKG: Option<&'static str> = option_env!("CARGO_PKG_NAME");

lazy_static! {
    /// stdout Drain switch
    pub static ref STDOUT_SW: AtomicSwitchCtrl<std::io::Error> = AtomicSwitch::new(
        level_filter(Level::Info, slog_term::streamer().async().compact().build())
    ).ctrl();
    /// stderr Drain switch
    pub static ref STDERR_SW: AtomicSwitchCtrl<std::io::Error> = AtomicSwitch::new(
        level_filter(Level::Info, slog_term::streamer().stderr().compact().build())
    ).ctrl();
}

/// Result used in mussh.
pub type MusshResult<T> = Result<T, MusshErr>;

/// mussh entry point
fn main() {
    process::exit(run::run(None));
}

#[cfg(test)]
mod main_test {
    use super::run;

    #[test]
    fn command_line() {
        assert!(0 == run::run(Some(vec!["mussh", "-vvvv", "--dryrun", "local", "python"])));
        assert!(0 ==
                run::run(Some(vec!["mussh",
                                   "--dryrun",
                                   "-c",
                                   "test_cfg/mussh.toml",
                                   "all",
                                   "python"])))
    }
}

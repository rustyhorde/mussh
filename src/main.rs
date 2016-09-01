// Copyright (c) 2016 goopy developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! mussh - SSH Multiplexing
#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![cfg_attr(feature="clippy", deny(clippy))]
#![deny(missing_docs)]
#![feature(question_mark)]

#[macro_use]
extern crate clap;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate slog;

extern crate rustc_serialize;
extern crate ssh2;
extern crate slog_term;
extern crate toml;

use error::MusshErr;
use slog::{AtomicSwitchCtrl, Level, async_stream, level_filter};
use std::io;
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
    pub static ref STDOUT_SW: AtomicSwitchCtrl = AtomicSwitchCtrl::new(
        level_filter(
            Level::Info,
            async_stream(io::stdout(), slog_term::format_colored())
        )
    );
    /// stderr Drain switch
    pub static ref STDERR_SW: AtomicSwitchCtrl = AtomicSwitchCtrl::new(
        async_stream(io::stderr(), slog_term::format_colored())
    );
}

/// Result used in mussh.
pub type MusshResult<T> = Result<T, MusshErr>;

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

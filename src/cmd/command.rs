// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `host` sub-command.
use clap::ArgMatches;
use config::Config;
use error::Result;
use slog::Logger;

/// Run the `host` sub-command.
pub fn cmd(_config: &mut Config,
           _sub_m: &ArgMatches,
           _stdout: &Logger,
           _stderr: &Logger)
           -> Result<i32> {
    Ok(0)
}

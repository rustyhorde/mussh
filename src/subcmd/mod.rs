// Copyright © 2016 libmussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! clap `SubCommand` modules
use crate::error::MusshResult;
use clap::{App, ArgMatches};
use libmussh::Config;

mod run;

pub(crate) use self::run::Run;

pub(crate) trait Subcommand {
    fn subcommand<'a, 'b>() -> App<'a, 'b>;
    fn execute(&self, config: &Config, matches: &ArgMatches<'_>) -> MusshResult<()>;
}

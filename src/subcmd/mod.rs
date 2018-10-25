crate mod run;

crate use self::run::Run;

use clap::{App, ArgMatches};
use failure::Fallible;

crate trait SubCmd {
    fn subcommand<'a, 'b>() -> App<'a, 'b>;
    fn cmd(matches: &ArgMatches<'_>) -> Fallible<()>;
}

crate mod run;

crate use self::run::Run;

use clap::App;
use failure::Fallible;

crate trait SubCmd {
    fn subcommand<'a, 'b>() -> App<'a, 'b>;
    fn cmd(&self) -> Fallible<()>;
}

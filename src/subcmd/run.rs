use clap::{App, Arg, ArgMatches, SubCommand};
use crate::subcmd::SubCmd;
use failure::Fallible;

crate struct Run;

impl SubCmd for Run {
    fn subcommand<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name("run")
            .about("Run a command on hosts")
            .arg(Arg::with_name("dry_run").long("dryrun").help(
                "Parse config and setup the client, \
                 but don't run it.",
            ))
            .arg(
                Arg::with_name("command")
                    .short("c")
                    .long("command")
                    .value_name("CMD")
                    .help("The command to multiplex")
                    .multiple(true)
                    .required(true),
            )
            .arg(
                Arg::with_name("hosts")
                    .short("h")
                    .long("hosts")
                    .value_name("HOSTS")
                    .help("The hosts to multiplex the command over")
                    .multiple(true)
                    .required(true),
            )
            .arg(Arg::with_name("sync").short("s").long("sync").help(
                "Run the given commadn synchronously across the \
                 hosts.",
            ))
    }

    fn cmd(_matches: &ArgMatches<'_>) -> Fallible<()> {
        println!("Running commands against host");
        Ok(())
    }
}

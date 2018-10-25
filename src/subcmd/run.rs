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
                Arg::with_name("commands")
                    .short("c")
                    .long("commands")
                    .value_name("CMD")
                    .help("The commands to multiplex")
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

    fn cmd(matches: &ArgMatches<'_>) -> Fallible<()> {
        let commands: Vec<_> = matches
            .values_of("commands")
            .ok_or_else(|| failure::err_msg("No commands found to run!"))?
            .collect();
        let hosts: Vec<_> = matches
            .values_of("hosts")
            .ok_or_else(|| failure::err_msg("No commands found to run!"))?
            .collect();
        let is_synchronous = matches.is_present("sync");

        println!(
            "Running '{}' against '{}' {}",
            commands.join(", "),
            hosts.join(", "),
            if is_synchronous {
                "synchronously"
            } else {
                "asynchronously"
            }
        );

        Ok(())
    }
}

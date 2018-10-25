use clap::{App, Arg, ArgMatches, SubCommand};
use crate::config::Mussh;
use crate::logging::Slogger;
use crate::subcmd::SubCmd;
use failure::{Error, Fallible};
use getset::{Getters, Setters};
use slog::{info, Logger};
use slog_try::try_info;
use std::convert::TryFrom;

#[derive(Clone, Debug, Default, Getters, Setters)]
crate struct Run {
    commands: Vec<String>,
    hosts: Vec<String>,
    sync: bool,
    stdout: Option<Logger>,
    stderr: Option<Logger>,
    #[set = "pub"]
    config: Option<Mussh>,
}

impl Slogger for Run {
    fn set_stdout(mut self, stdout: Option<Logger>) -> Self {
        self.stdout = stdout;
        self
    }

    fn set_stderr(mut self, stderr: Option<Logger>) -> Self {
        self.stderr = stderr;
        self
    }
}

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

    fn cmd(&self) -> Fallible<()> {
        try_info!(
            self.stdout,
            "Running '{}' against '{}' {}",
            self.commands.join(", "),
            self.hosts.join(", "),
            if self.sync {
                "synchronously"
            } else {
                "asynchronously"
            }
        );

        Ok(())
    }
}

impl<'a> TryFrom<&'a ArgMatches<'a>> for Run {
    type Error = Error;

    fn try_from(matches: &'a ArgMatches<'a>) -> Fallible<Self> {
        let mut run = Self::default();
        run.commands = matches
            .values_of("commands")
            .ok_or_else(|| failure::err_msg("No commands found to run!"))?
            .map(|s| s.to_string())
            .collect();
        run.hosts = matches
            .values_of("hosts")
            .ok_or_else(|| failure::err_msg("No commands found to run!"))?
            .map(|s| s.to_string())
            .collect();
        run.sync = matches.is_present("sync");
        Ok(run)
    }
}

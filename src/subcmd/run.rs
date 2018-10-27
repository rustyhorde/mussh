use clap::{App, Arg, ArgMatches, SubCommand};
use crate::config::Mussh;
use crate::error::MusshErrorKind;
use crate::logging::Slogger;
use crate::subcmd::SubCmd;
use failure::{Error, Fallible};
use getset::{Getters, Setters};
use slog::{trace, warn, Logger};
use slog_try::{try_trace, try_warn};
use std::collections::HashSet;
use std::convert::TryFrom;
use std::iter::FromIterator;

#[derive(Clone, Debug, Default, Getters, Setters)]
crate struct Run {
    commands: Vec<String>,
    hosts: Vec<String>,
    sync: bool,
    stdout: Option<Logger>,
    stderr: Option<Logger>,
    #[set = "pub"]
    #[get = "pub"]
    config: Option<Mussh>,
    #[set = "pub"]
    #[get = "pub"]
    dry_run: bool,
}

fn hostnames(config: &Mussh, host: &str) -> Vec<String> {
    if let Some(hosts) = config.hostlist().get(host) {
        hosts.hostnames().clone()
    } else {
        vec![]
    }
}

fn unwanted_host(host: &str) -> Option<String> {
    if host.starts_with('!') {
        Some((*host).split_at(1).1.to_string())
    } else {
        None
    }
}

impl Run {
    fn display_set(&self, label: &str, hash_set: &HashSet<String>, trace: bool) {
        let set_str = hash_set.iter().cloned().collect::<Vec<String>>().join(", ");

        if trace {
            try_trace!(self.stdout, "{}'{}'", label, set_str);
        } else {
            try_warn!(
                self.stdout,
                "The given hosts '{}' are not configured!",
                set_str
            );
        }
    }

    fn target_hosts(&self) -> Fallible<HashSet<String>> {
        if let Some(config) = self.config() {
            let requested_hosts: HashSet<String> = HashSet::from_iter(self.hosts.iter().cloned());
            self.display_set("Command Line Hosts:      ", &requested_hosts, true);

            let mut expanded_hosts: HashSet<String> =
                HashSet::from_iter(self.hosts.iter().flat_map(|host| hostnames(config, host)));
            self.display_set("Expanded Hosts:          ", &expanded_hosts, true);

            let remove_unwanted: HashSet<String> =
                HashSet::from_iter(self.hosts.iter().filter_map(|host| unwanted_host(host)));
            self.display_set("Unwanted Hosts:          ", &remove_unwanted, true);

            expanded_hosts.retain(|x| !remove_unwanted.contains(x));

            let configured_hosts: HashSet<String> =
                HashSet::from_iter(config.hostlist().keys().cloned());
            self.display_set("Configured Hosts:        ", &configured_hosts, true);

            let not_configured_hosts: HashSet<String> = expanded_hosts
                .difference(&configured_hosts)
                .cloned()
                .collect();
            self.display_set("Not Configured Hosts:    ", &not_configured_hosts, true);

            if !not_configured_hosts.is_empty() {
                self.display_set("", &not_configured_hosts, false);
            }

            let matched_hosts: HashSet<String> = expanded_hosts
                .intersection(&configured_hosts)
                .cloned()
                .collect();
            self.display_set("Matched Hosts:           ", &matched_hosts, true);

            Ok(matched_hosts)
        } else {
            Err(MusshErrorKind::InvalidConfigToml.into())
        }
    }

    fn cmds(&self) -> Fallible<String> {
        if let Some(_config) = self.config() {
            Err(failure::err_msg("not implemented!"))
        } else {
            Err(MusshErrorKind::InvalidConfigToml.into())
        }
    }
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
        let _target_hosts = self.target_hosts()?;
        let _cmds = self.cmds()?;
        if self.dry_run {
            // TODO: output what would have run
            println!("Dry Run");
            Ok(())
        } else {
            Ok(())
        }
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

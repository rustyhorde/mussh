use clap::{App, Arg, ArgMatches, SubCommand};
use crate::config::{Command, Host, Mussh};
use crate::error::MusshErrorKind;
use crate::logging::{FileDrain, Slogger};
use crate::subcmd::SubCmd;
use failure::{Error, Fallible};
use getset::{Getters, Setters};
use slog::{error, info, o, trace, warn, Drain, Logger};
use slog_try::{try_error, try_info, try_trace, try_warn};
use std::collections::{BTreeMap, HashSet};
use std::convert::TryFrom;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::path::PathBuf;
use std::process::{Command as Cmd, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

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

enum WarnType {
    Hosts,
    Cmds,
}

impl Run {
    fn display_set(&self, label: &str, hash_set: &HashSet<String>, warn_type: &Option<WarnType>) {
        let set_str = hash_set.iter().cloned().collect::<Vec<String>>().join(", ");

        if warn_type.is_none() {
            try_trace!(self.stdout, "{}'{}'", label, set_str);
        } else {
            let type_str = match warn_type {
                Some(WarnType::Hosts) => "hosts",
                Some(WarnType::Cmds) => "commands",
                _ => "warn_type unk",
            };
            try_warn!(
                self.stdout,
                "The given {} '{}' are not configured!",
                type_str,
                set_str
            );
        }
    }

    fn target_hosts(&self) -> Fallible<BTreeMap<String, Host>> {
        if let Some(config) = self.config() {
            let requested_hosts: HashSet<String> = HashSet::from_iter(self.hosts.iter().cloned());
            self.display_set("Command Line Hosts:      ", &requested_hosts, &None);

            let mut expanded_hosts: HashSet<String> =
                HashSet::from_iter(self.hosts.iter().flat_map(|host| hostnames(config, host)));
            self.display_set("Expanded Hosts:          ", &expanded_hosts, &None);

            let remove_unwanted: HashSet<String> =
                HashSet::from_iter(self.hosts.iter().filter_map(|host| unwanted_host(host)));
            self.display_set("Unwanted Hosts:          ", &remove_unwanted, &None);

            expanded_hosts.retain(|x| !remove_unwanted.contains(x));

            let configured_hosts: HashSet<String> =
                HashSet::from_iter(config.hostlist().keys().cloned());
            self.display_set("Configured Hosts:        ", &configured_hosts, &None);

            let not_configured_hosts: HashSet<String> = expanded_hosts
                .difference(&configured_hosts)
                .cloned()
                .collect();
            self.display_set("Not Configured Hosts:    ", &not_configured_hosts, &None);

            if !not_configured_hosts.is_empty() {
                self.display_set("", &not_configured_hosts, &Some(WarnType::Hosts));
            }

            let matched_hosts: BTreeMap<String, Host> = expanded_hosts
                .intersection(&configured_hosts)
                .filter_map(|hostname| {
                    if let Some(host) = config.hosts().get(hostname) {
                        Some((hostname.clone(), host.clone()))
                    } else {
                        None
                    }
                })
                .collect();

            Ok(matched_hosts)
        } else {
            Err(MusshErrorKind::InvalidConfigToml.into())
        }
    }

    fn target_cmds(&self) -> Fallible<BTreeMap<String, Command>> {
        if let Some(config) = self.config() {
            let requested_cmds: HashSet<String> = HashSet::from_iter(self.commands.iter().cloned());
            self.display_set("Command Line Commands:   ", &requested_cmds, &None);

            let configured_cmds: HashSet<String> = HashSet::from_iter(config.cmd().keys().cloned());
            self.display_set("Configured Commands:     ", &configured_cmds, &None);

            let not_configured_commands: HashSet<String> = requested_cmds
                .difference(&configured_cmds)
                .cloned()
                .collect();
            self.display_set("Not Configured Commands: ", &not_configured_commands, &None);

            if !not_configured_commands.is_empty() {
                self.display_set("", &not_configured_commands, &Some(WarnType::Cmds));
            }

            let matched_cmds: BTreeMap<String, Command> = requested_cmds
                .intersection(&configured_cmds)
                .filter_map(|cmd_name| {
                    if let Some(cmd) = config.cmd().get(cmd_name) {
                        Some((cmd_name.clone(), cmd.clone()))
                    } else {
                        try_warn!(self.stdout, "{} is not configured!", cmd_name);
                        None
                    }
                })
                .collect();
            Ok(matched_cmds)
        } else {
            Err(MusshErrorKind::InvalidConfigToml.into())
        }
    }

    fn actual_cmds(
        &self,
        target_host: &Host,
        expected_cmds: &BTreeMap<String, Command>,
    ) -> Fallible<BTreeMap<String, String>> {
        if let Some(config) = self.config() {
            Ok(expected_cmds
                .iter()
                .map(|(cmd_name, command)| {
                    (
                        cmd_name.clone(),
                        if let Some(alias_vec) = target_host.alias() {
                            let mut cmd = command.command().clone();
                            for alias in alias_vec {
                                if alias.aliasfor() == cmd_name {
                                    if let Some(int_command) = config.cmd().get(alias.command()) {
                                        cmd = int_command.command().clone();
                                        break;
                                    }
                                }
                            }
                            cmd
                        } else {
                            command.command().clone()
                        },
                    )
                })
                .collect())
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
        let target_hosts = self.target_hosts()?;
        let count = target_hosts.len();
        let cmds = self.target_cmds()?;
        let (tx, rx) = mpsc::channel();

        for (hostname, host) in target_hosts {
            let actual_cmds: BTreeMap<String, String> = self.actual_cmds(&host, &cmds)?;

            try_trace!(
                self.stdout,
                "Executing {:?} on {}",
                actual_cmds.keys(),
                hostname
            );

            if !self.dry_run {
                let h_tx = tx.clone();
                let stdout_t = self.stdout.clone();
                let stderr_t = self.stdout.clone();
                let _ = thread::spawn(move || {
                    h_tx.send(execute(
                        &stdout_t,
                        &stderr_t,
                        &hostname,
                        &host,
                        &actual_cmds,
                    ))
                    .expect("Unable to send response!");
                });
            }
        }

        if !self.dry_run {
            for _ in 0..count {
                match rx.recv() {
                    Ok(res) => {
                        if let Err(e) = res {
                            try_error!(self.stderr, "{}", e);
                        }
                    }
                    Err(e) => {
                        try_error!(self.stderr, "{}", e);
                    }
                }
            }
        }
        Ok(())
    }
}

fn convert_duration(duration: Duration) -> String {
    if duration.as_secs() < 1 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{}.{}s", duration.as_secs(), duration.subsec_millis())
    }
}

fn execute(
    stdout: &Option<Logger>,
    stderr: &Option<Logger>,
    hostname: &str,
    host: &Host,
    cmds: &BTreeMap<String, String>,
) -> Fallible<()> {
    let mut host_file_path = if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push(env!("CARGO_PKG_NAME"));
        config_dir
    } else {
        PathBuf::new()
    };

    host_file_path.push(hostname);
    let _ = host_file_path.set_extension("log");

    try_trace!(stdout, "Log Path: {}", host_file_path.display());

    let file_drain = FileDrain::try_from(host_file_path)?;
    let async_file_drain = slog_async::Async::new(file_drain).build().fuse();
    let file_logger = Logger::root(async_file_drain, o!());
    let timer = Instant::now();

    if host.hostname() == "localhost" {
        for (cmd_name, cmd) in cmds {
            let mut command = Cmd::new("/usr/bin/fish");
            let _ = command.arg("-c");
            let _ = command.arg(cmd);
            let _ = command.stdout(Stdio::piped());
            let _ = command.stderr(Stdio::piped());

            if let Ok(mut child) = command.spawn() {
                let stdout_reader = BufReader::new(child.stdout.take().expect(""));
                for line in stdout_reader.lines() {
                    if let Ok(line) = line {
                        trace!(file_logger, "{}", line);
                    }
                }

                let status = child.wait()?;
                let elapsed_str = convert_duration(timer.elapsed());

                if status.success() {
                    try_info!(
                        stdout,
                        "execute";
                        "host" => host.hostname(),
                        "cmd" => cmd_name,
                        "duration" => elapsed_str
                    );
                } else {
                    try_error!(
                        stderr,
                        "execute";
                        "host" => host.hostname(),
                        "cmd" => cmd_name,
                        "duration" => elapsed_str
                    );
                }
            }
        }
        Ok(())
    } else {
        Err(failure::err_msg("not implemented!"))
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

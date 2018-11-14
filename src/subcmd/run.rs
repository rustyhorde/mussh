use clap::{App, Arg, ArgMatches, SubCommand};
use crate::config::{Command, Host, Mussh};
use crate::error::MusshErrorKind;
use crate::logging::{FileDrain, Slogger};
use crate::subcmd::SubCmd;
use crossbeam::sync::WaitGroup;
use failure::{Error, Fallible};
use getset::{Getters, Setters};
use indexmap::{IndexMap, IndexSet};
use slog::{error, info, o, trace, warn, Drain, Logger};
use slog_try::{try_error, try_info, try_trace, try_warn};
use ssh2::Session;
use std::convert::TryFrom;
use std::env;
use std::io::{BufRead, BufReader};
use std::iter::FromIterator;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command as Cmd, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

const CMD_LINE_HOSTS: &str = "Command Line Hosts:      ";
const CMD_LINE_PRE_HOSTS: &str = "Command Line Pre-Hosts:  ";
const EXPANDED_HOSTS: &str = "Expanded Hosts:          ";
const EXPANDED_PRE_HOSTS: &str = "Expanded Pre-Hosts:      ";
const UNWANTED_HOSTS: &str = "Unwanted Hosts:          ";
const UNWANTED_PRE_HOSTS: &str = "Unwanted Pre-Hosts:      ";
const CONFIGURED_HOSTS: &str = "Configured Hosts:        ";
const CONFIGURED_PRE_HOSTS: &str = "Configured Pre-Hosts     ";
const UNCONFIGURED_HOSTS: &str = "Unconfigured Hosts:      ";
const UNCONFIGURED_PRE_HOSTS: &str = "Unconfigured Pre-Hosts   ";

#[derive(Clone, Debug, Default, Getters, Setters)]
crate struct Run {
    #[get]
    commands: Vec<String>,
    #[get]
    hosts: Vec<String>,
    #[get]
    group_cmds: Vec<String>,
    #[get]
    group_pre: Vec<String>,
    #[get]
    group: Vec<String>,
    sync: bool,
    stdout: Option<Logger>,
    stderr: Option<Logger>,
    #[get]
    #[set = "pub"]
    config: Mussh,
    #[set = "pub"]
    dry_run: bool,
}

fn hostnames(config: &Mussh, host: &str) -> Vec<String> {
    config
        .hostlist()
        .get(host)
        .map_or_else(|| vec![], |hosts| hosts.hostnames().clone())
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

enum HostType {
    HOST,
    PRE,
}

impl Run {
    fn display_set(&self, label: &str, hash_set: &IndexSet<String>, warn_type: &Option<WarnType>) {
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

    fn as_set<T>(&self, iter: T, prefix: &str, warn_type: &Option<WarnType>) -> IndexSet<String>
    where
        T: IntoIterator<Item = String>,
    {
        let set = IndexSet::from_iter(iter);
        if !set.is_empty() {
            self.display_set(prefix, &set, warn_type);
        }
        set
    }

    fn requested(&self, host_type: &HostType) -> IndexSet<String> {
        let (hosts, prefix) = match host_type {
            HostType::HOST => (self.hosts(), CMD_LINE_HOSTS),
            HostType::PRE => (self.group_pre(), CMD_LINE_PRE_HOSTS),
        };
        self.as_set(hosts.iter().cloned(), prefix, &None)
    }

    fn expanded(&self, host_type: &HostType) -> IndexSet<String> {
        let (hosts, prefix) = match host_type {
            HostType::HOST => (self.hosts(), EXPANDED_HOSTS),
            HostType::PRE => (self.group_pre(), EXPANDED_PRE_HOSTS),
        };
        let hostnames = hosts.iter().flat_map(|host| hostnames(&self.config, host));
        self.as_set(hostnames, prefix, &None)
    }

    fn unwanted(&self, host_type: &HostType) -> IndexSet<String> {
        let (hosts, prefix) = match host_type {
            HostType::HOST => (self.hosts(), UNWANTED_HOSTS),
            HostType::PRE => (self.group_pre(), UNWANTED_PRE_HOSTS),
        };
        let unwanted = hosts.iter().filter_map(|host| unwanted_host(host));
        self.as_set(unwanted, prefix, &None)
    }

    fn configured(&self, host_type: &HostType) -> IndexSet<String> {
        let (hosts, prefix) = match host_type {
            HostType::HOST => (self.config().hostlist(), CONFIGURED_HOSTS),
            HostType::PRE => (self.config().hostlist(), CONFIGURED_PRE_HOSTS),
        };
        let configured = hosts.keys().cloned();
        self.as_set(configured, prefix, &None)
    }

    fn unconfigured(
        &self,
        host_type: &HostType,
        expanded: &IndexSet<String>,
        configured: &IndexSet<String>,
    ) -> IndexSet<String> {
        let prefix = match host_type {
            HostType::HOST => UNCONFIGURED_HOSTS,
            HostType::PRE => UNCONFIGURED_PRE_HOSTS,
        };
        let unconfigured = expanded.difference(configured).cloned();
        self.as_set(unconfigured, prefix, &Some(WarnType::Hosts))
    }

    fn pre_hosts(&self) -> IndexSet<String> {
        let host_type = HostType::PRE;
        // Genereate the set of hosts that were requested
        let _ = self.requested(&host_type);
        // Generate the set of expanded hosts
        let mut expanded = self.expanded(&host_type);
        // Generate the set of unwanted hosts
        let unwanted = self.unwanted(&host_type);
        // Retain the wanted hosts from the expanded set
        expanded.retain(|x| !unwanted.contains(x));
        // Generate the set of hosts that are configured in mussh.toml.
        let configured = self.configured(&host_type);
        // Generate the set of hosts that were requested and not configured.
        let _ = self.unconfigured(&host_type, &expanded, &configured);
        // Generate the set of hosts that were requested and configured.
        expanded.intersection(&configured).cloned().collect()
    }

    fn target_hosts(&self) -> IndexSet<String> {
        let host_type = HostType::HOST;
        // Genereate the set of hosts that were requested
        let _ = self.requested(&host_type);
        // Generate the set of expanded hosts
        let mut expanded = self.expanded(&host_type);
        // Generate the set of unwanted hosts
        let unwanted = self.unwanted(&host_type);
        // Retain the wanted hosts from the expanded set
        expanded.retain(|x| !unwanted.contains(x));
        // Generate the set of hosts that are configured in mussh.toml.
        let configured = self.configured(&host_type);
        // Generate the set of hosts that were requested and not configured.
        let _ = self.unconfigured(&host_type, &expanded, &configured);
        // Generate the set of hosts that were requested and configured.
        expanded.intersection(&configured).cloned().collect()
    }

    fn target_cmds(&self) -> IndexMap<String, Command> {
        let requested_cmds: IndexSet<String> = IndexSet::from_iter(self.commands().iter().cloned());
        self.display_set("Command Line Commands:   ", &requested_cmds, &None);

        let configured_cmds: IndexSet<String> =
            IndexSet::from_iter(self.config().cmd().keys().cloned());
        self.display_set("Configured Commands:     ", &configured_cmds, &None);

        let not_configured_commands: IndexSet<String> = requested_cmds
            .difference(&configured_cmds)
            .cloned()
            .collect();
        self.display_set("Not Configured Commands: ", &not_configured_commands, &None);

        if !not_configured_commands.is_empty() {
            self.display_set("", &not_configured_commands, &Some(WarnType::Cmds));
        }

        requested_cmds
            .intersection(&configured_cmds)
            .filter_map(|cmd_name| {
                self.config()
                    .cmd()
                    .get(cmd_name)
                    .and_then(|cmd| Some((cmd_name.clone(), cmd.clone())))
            })
            .collect()
    }

    fn actual_cmds(
        &self,
        target_host: &Host,
        expected_cmds: &IndexMap<String, Command>,
    ) -> Fallible<IndexMap<String, String>> {
        Ok(expected_cmds
            .iter()
            .map(|(cmd_name, command)| setup_alias(self.config(), command, cmd_name, target_host))
            .collect())
    }

    fn host_file_logger(&self, hostname: &str) -> Fallible<Logger> {
        let mut host_file_path = if let Some(mut config_dir) = dirs::config_dir() {
            config_dir.push(env!("CARGO_PKG_NAME"));
            config_dir
        } else {
            PathBuf::new()
        };

        host_file_path.push(hostname);
        let _ = host_file_path.set_extension("log");

        try_trace!(self.stdout, "Log Path: {}", host_file_path.display());

        let file_drain = FileDrain::try_from(host_file_path)?;
        let async_file_drain = slog_async::Async::new(file_drain).build().fuse();
        let file_logger = Logger::root(async_file_drain, o!());
        Ok(file_logger)
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
                    .required_unless("group_cmds"),
            )
            .arg(
                Arg::with_name("hosts")
                    .short("h")
                    .long("hosts")
                    .value_name("HOSTS")
                    .help("The hosts to multiplex the command over")
                    .multiple(true)
                    .required_unless("group"),
            )
            .arg(
                Arg::with_name("group_cmds")
                    .long("group-cmds")
                    .value_name("GROUP_CMDS")
                    .help("The commands to multiplex after completing on the pre-group")
                    .use_delimiter(true)
                    .required_unless("hosts")
                    .requires_all(&["group_pre", "group"])
            )
            .arg(
                Arg::with_name("group_pre")
                    .long("group-pre")
                    .value_name("GROUP_PRE")
                    .help("The group of hosts that should complete before moving on to the hosts specified by the 'group' flag")
                    .use_delimiter(true)
                    .requires("group_cmds")
            )
            .arg(
                Arg::with_name("group")
                    .long("group")
                    .value_name("GROUP")
                    .help("The group of hosts that should run after the hosts specified by the 'group-pre' flag")
                    .use_delimiter(true)
                    .requires("group_cmds")
            )
            .arg(Arg::with_name("sync").short("s").long("sync").help(
                "Run the given commadn synchronously across the \
                 hosts.",
            ))
    }

    fn multiplex(&self) -> Fallible<()> {
        try_trace!(self.stdout, "Multiplexing commands across hosts");
        let target_hosts = self.target_hosts();
        let pre_hosts = self.pre_hosts();
        let count = target_hosts.len();
        let cmds = self.target_cmds();
        let (tx, rx) = mpsc::channel();
        let wg = WaitGroup::new();

        let matched_hosts: IndexMap<String, Host> = target_hosts
            .union(&pre_hosts)
            .filter_map(|hostname| {
                self.config()
                    .hosts()
                    .get(hostname)
                    .and_then(|host| Some((hostname.clone(), host.clone())))
            })
            .collect();

        for (hostname, host) in matched_hosts {
            let actual_cmds: IndexMap<String, String> = self.actual_cmds(&host, &cmds)?;

            try_trace!(
                self.stdout,
                "Executing {} on {}",
                actual_cmds
                    .keys()
                    .cloned()
                    .collect::<Vec<String>>()
                    .join(","),
                hostname
            );

            let file_logger = self.host_file_logger(&hostname)?;

            if !self.dry_run {
                let h_tx = tx.clone();
                let stdout_t = self.stdout.clone();
                let stderr_t = self.stdout.clone();
                let file_logger_t = file_logger.clone();

                let _ = thread::spawn(move || {
                    h_tx.send(execute(
                        &stdout_t,
                        &stderr_t,
                        &file_logger_t,
                        &host,
                        &actual_cmds,
                    ))
                    .expect("Unable to send response!");
                });
            }

            if self.sync {
                match rx.recv() {
                    Ok(results) => {
                        for (cmd_name, (hostname, res)) in results {
                            if let Err(e) = res {
                                try_error!(
                                    self.stderr,
                                    "Failed to run '{}' on '{}': {}",
                                    cmd_name,
                                    hostname,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        try_error!(self.stderr, "{}", e);
                    }
                }
            }
        }

        if !self.dry_run && !self.sync {
            for _ in 0..count {
                match rx.recv() {
                    Ok(results) => {
                        for (cmd_name, (hostname, res)) in results {
                            if let Err(e) = res {
                                try_error!(
                                    self.stderr,
                                    "Failed to run '{}' on '{}': {}",
                                    cmd_name,
                                    hostname,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        try_error!(self.stderr, "{}", e);
                    }
                }
            }
        }
        wg.wait();
        Ok(())
    }
}

fn setup_alias(
    config: &Mussh,
    command: &Command,
    cmd_name: &str,
    target_host: &Host,
) -> (String, String) {
    (
        cmd_name.to_string(),
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
}

fn convert_duration(duration: Duration) -> String {
    let seconds = duration.as_secs();
    let millis = duration.subsec_millis();
    if seconds < 1 {
        format!("00:00:00.{:03}", duration.as_millis())
    } else if seconds < 60 {
        format!("00:00:{:02}.{:03}", seconds, millis)
    } else if seconds < 3600 {
        let minutes = seconds / 60;
        let seconds = seconds % 60;
        format!("00:{:02}:{:02}.{:03}", minutes, seconds, millis)
    } else if seconds < 86400 {
        let total_minutes = seconds / 60;
        let seconds = seconds % 60;
        let hours = total_minutes / 60;
        let minutes = total_minutes % 60;
        format!("{}:{:02}:{:02}.{:03}", hours, minutes, seconds, millis)
    } else {
        format!("{}s", seconds)
    }
}

#[allow(dead_code)]
fn which<P>(exe_name: P) -> Option<PathBuf>
where
    P: AsRef<Path>,
{
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths)
            .filter_map(|dir| {
                let full_path = dir.join(&exe_name);
                if full_path.is_file() {
                    Some(full_path)
                } else {
                    None
                }
            })
            .next()
    })
}

fn execute_on_localhost(
    stdout: &Option<Logger>,
    stderr: &Option<Logger>,
    file_logger: &Logger,
    host: &Host,
    cmd_name: &str,
    cmd: &str,
) -> Fallible<()> {
    if let Some(shell_path) = env::var_os("SHELL") {
        let timer = Instant::now();
        let fish = shell_path.to_string_lossy().to_string();
        let mut command = Cmd::new(&fish);
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
        Ok(())
    } else {
        Err(MusshErrorKind::ShellNotFound.into())
    }
}

fn execute_on_remote(
    stdout: &Option<Logger>,
    stderr: &Option<Logger>,
    file_logger: &Logger,
    host: &Host,
    cmd_name: &str,
    cmd: &str,
) -> Fallible<()> {
    if let Some(mut sess) = Session::new() {
        let timer = Instant::now();
        let host_tuple = (&host.hostname()[..], host.port().unwrap_or_else(|| 22));
        let tcp = TcpStream::connect(host_tuple)?;
        sess.handshake(&tcp)?;
        if let Some(pem) = host.pem() {
            sess.userauth_pubkey_file(host.username(), None, Path::new(&pem), None)?;
        } else {
            try_trace!(stdout, "execute"; "message" => "Agent Auth Setup", "username" => host.username());
            let mut agent = sess.agent()?;
            agent.connect()?;
            agent.list_identities()?;
            for identity in agent.identities() {
                if let Ok(ref id) = identity {
                    if agent.userauth(host.username(), id).is_ok() {
                        break;
                    }
                }
            }
            agent.disconnect()?;
        }

        if sess.authenticated() {
            try_trace!(stdout, "execute"; "message" => "Authenticated");
            let mut channel = sess.channel_session()?;
            channel.exec(cmd)?;

            {
                let stdout_stream = channel.stream(0);
                let stdout_reader = BufReader::new(stdout_stream);

                for line in stdout_reader.lines() {
                    if let Ok(line) = line {
                        trace!(file_logger, "{}", line);
                    }
                }
            }

            let elapsed_str = convert_duration(timer.elapsed());
            match channel.exit_status() {
                Ok(code) => {
                    if code == 0 {
                        try_info!(
                            stdout,
                            "execute";
                            "host" => host.hostname(),
                            "cmd" => cmd_name,
                            "duration" => elapsed_str
                        );
                        Ok(())
                    } else {
                        try_error!(
                            stderr,
                            "execute";
                            "host" => host.hostname(),
                            "cmd" => cmd_name,
                            "duration" => elapsed_str
                        );
                        Err(failure::err_msg("ssh cmd failed"))
                    }
                }
                Err(e) => {
                    try_error!(
                        stderr,
                        "execute"; "hostname" => host.hostname(), "cmd" => cmd_name, "error" => format!("{}", e)
                    );
                    Err(e.into())
                }
            }
        } else {
            Err(MusshErrorKind::SshAuthentication.into())
        }
    } else {
        Err(MusshErrorKind::SshSession.into())
    }
}

fn execute_on_host(
    stdout: &Option<Logger>,
    stderr: &Option<Logger>,
    file_logger: &Logger,
    host: &Host,
    cmd_name: &str,
    cmd: &str,
) -> Fallible<()> {
    if host.hostname() == "localhost" {
        execute_on_localhost(stdout, stderr, file_logger, host, cmd_name, cmd)
    } else {
        execute_on_remote(stdout, stderr, file_logger, host, cmd_name, cmd)
    }
}

fn execute(
    stdout: &Option<Logger>,
    stderr: &Option<Logger>,
    file_logger: &Logger,
    host: &Host,
    cmds: &IndexMap<String, String>,
) -> IndexMap<String, (String, Fallible<()>)> {
    cmds.iter()
        .map(|(cmd_name, cmd)| {
            (
                cmd_name.clone(),
                (
                    host.hostname().clone(),
                    execute_on_host(stdout, stderr, file_logger, host, cmd_name, cmd),
                ),
            )
        })
        .collect()
}

impl<'a> TryFrom<&'a ArgMatches<'a>> for Run {
    type Error = Error;

    fn try_from(matches: &'a ArgMatches<'a>) -> Fallible<Self> {
        let mut run = Self::default();
        run.commands = matches
            .values_of("commands")
            .map_or_else(|| vec![], |values| values.map(|v| v.to_string()).collect());
        run.hosts = matches
            .values_of("hosts")
            .map_or_else(|| vec![], |values| values.map(|v| v.to_string()).collect());
        run.group_pre = matches
            .values_of("group_pre")
            .map_or_else(|| vec![], |values| values.map(|v| v.to_string()).collect());
        run.group_cmds = matches
            .values_of("group_cmds")
            .map_or_else(|| vec![], |values| values.map(|v| v.to_string()).collect());
        run.group = matches
            .values_of("group")
            .map_or_else(|| vec![], |values| values.map(|v| v.to_string()).collect());
        run.sync = matches.is_present("sync");
        Ok(run)
    }
}

#[cfg(test)]
mod test {
    use super::convert_duration;
    use std::time::Duration;

    #[test]
    fn conversions() {
        assert_eq!(convert_duration(Duration::from_millis(876)), "00:00:00.876");
        assert_eq!(
            convert_duration(Duration::from_millis(10432)),
            "00:00:10.432"
        );
        assert_eq!(
            convert_duration(Duration::from_millis(2_421_132)),
            "00:40:21.132"
        );
        assert_eq!(
            convert_duration(Duration::from_millis(12_423_756)),
            "3:27:03.756"
        );
    }
}

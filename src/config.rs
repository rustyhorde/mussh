// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `mussh` config.
use chrono::{DateTime, UTC};
use error::{ErrorKind, Result};
use slog::{Drain, Level, LevelFilter, Logger, Never, OwnedKVList, Record};
use slog_async;
use slog_term;
use std::collections::HashMap;
use std::{env, fmt};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use toml;

/// Default configuration filename.
pub const CONFIG_FILE_NAME: &'static str = "mussh.toml";
/// Default 'dot' directory for `mussh` configuration.
pub const DOT_DIR: &'static str = ".mussh";

/// `mussh` Config
#[derive(Clone)]
pub struct Config {
    /// Non-standard directory for the TOML config.
    toml_dir: Option<PathBuf>,
    /// The command being multiplexed.
    cmd: String,
    /// The hosts to run the command on.
    hosts: Vec<String>,
    /// The TOML config.
    toml: Option<MusshToml>,
    /// The slog stdout `Logger`.
    stdout: Logger,
    /// The slog stderr `Logger`.
    stderr: Logger,
}

impl Config {
    /// Get the `toml_dir` value.
    pub fn toml_dir(&self) -> Option<PathBuf> {
        if let Some(ref pb) = self.toml_dir {
            Some(pb.clone())
        } else {
            None
        }
    }

    /// Set the `toml_dir` value.
    pub fn set_toml_dir(&mut self, toml_dir: &str) -> &mut Config {
        self.toml_dir = Some(PathBuf::from(toml_dir));
        self
    }

    /// Get the `cmd` value.
    pub fn cmd(&self) -> &str {
        &self.cmd
    }

    /// Set the `cmd` value.
    pub fn set_cmd(&mut self, cmd: &str) -> &mut Config {
        self.cmd = cmd.to_string();
        self
    }

    /// Get the `hosts` value.
    pub fn hosts(&self) -> Vec<&str> {
        self.hosts.iter().map(|x| &**x).collect()
    }

    /// Set the `hosts` value.
    pub fn set_hosts(&mut self, hosts: Vec<&str>) -> &mut Config {
        self.hosts = hosts.iter().map(|x| x.to_string()).collect();
        self
    }

    /// Get the `toml` value.
    pub fn toml(&self) -> Option<MusshToml> {
        if let Some(ref toml) = self.toml {
            Some(toml.clone())
        } else {
            None
        }
    }

    /// Set the `toml` value.
    pub fn set_toml(&mut self, toml: MusshToml) -> &mut Config {
        self.toml = Some(toml);
        self
    }

    /// Get the `stdout` value.
    pub fn stdout(&self) -> Logger {
        self.stdout.clone()
    }

    /// Set the stdout slog 'Logger' level.
    pub fn set_stdout_level(&mut self, level: Level) -> &mut Config {
        self.stdout = stdout_logger(level);
        self
    }

    /// Get the `stderr` value.
    pub fn stderr(&self) -> Logger {
        self.stderr.clone()
    }
}

impl Default for Config {
    fn default() -> Config {
        Config {
            toml_dir: None,
            cmd: String::new(),
            hosts: Vec::new(),
            toml: None,
            stdout: stdout_logger(Level::Error),
            stderr: stderr_logger(),
        }
    }
}

/// Setup the stderr slog `Logger`
fn stderr_logger() -> Logger {
    let stderr_decorator = slog_term::TermDecorator::new().stderr().build();
    let stderr_drain = slog_term::CompactFormat::new(stderr_decorator)
        .build()
        .fuse();
    let stderr_async_drain = slog_async::Async::new(stderr_drain).build().fuse();
    let stderr_level_drain = LevelFilter::new(stderr_async_drain, Level::Error).fuse();
    Logger::root(stderr_level_drain,
                 o!(
        "executable" => env!("CARGO_PKG_NAME"),
        "version" => env!("CARGO_PKG_VERSION")
    ))
}

/// Setup the stdout slog `Logger`
fn stdout_logger(level: Level) -> Logger {
    let stdout_decorator = slog_term::TermDecorator::new().stdout().build();
    let stdout_drain = slog_term::CompactFormat::new(stdout_decorator)
        .build()
        .fuse();
    let stdout_async_drain = slog_async::Async::new(stdout_drain).build().fuse();
    let stdout_level_drain = LevelFilter::new(stdout_async_drain, level).fuse();
    Logger::root(stdout_level_drain,
                 o!(
        "executable" => env!("CARGO_PKG_NAME"),
        "version" => env!("CARGO_PKG_VERSION")
    ))
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// The base configuration.
pub struct MusshToml {
    /// A list of hosts.
    hostlist: Option<HashMap<String, Hosts>>,
    /// The hosts.
    hosts: Option<HashMap<String, Host>>,
    /// A command.
    cmd: Option<HashMap<String, Command>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// hosts configuration
pub struct Hosts {
    /// The hostnames.
    hostnames: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Host configuration.
pub struct Host {
    /// A hostname.
    hostname: String,
    /// A pem key.
    pem: Option<String>,
    /// A port
    port: Option<u16>,
    /// A username.
    username: String,
    /// A command alias.
    alias: Option<Vec<Alias>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// command configuration
pub struct Command {
    /// A Command.
    command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// command alias configuration.
pub struct Alias {
    /// A command alias.
    command: String,
    /// The command this is an alias for.
    aliasfor: String,
}

impl MusshToml {
    /// Create a new 'MusshToml' from mussh.toml on the filesystem.
    pub fn new(toml_dir: Option<PathBuf>) -> Result<MusshToml> {
        for path in &paths(toml_dir) {
            if let Ok(mut config_file) = File::open(path) {
                let mut toml_buf = vec![];
                if config_file.read_to_end(&mut toml_buf).is_ok() {
                    let toml_str = String::from_utf8_lossy(&toml_buf);
                    if let Ok(parsed) = toml::from_str(&toml_str) {
                        return Ok(parsed);
                    }
                }
            }
        }
        Err(ErrorKind::Config.into())
    }

    /// Get the `hostlist` value.
    pub fn hostlist(&self) -> Option<&HashMap<String, Hosts>> {
        match self.hostlist {
            Some(ref h) => Some(h),
            None => None,
        }
    }

    /// Get the `hosts` value.
    pub fn hosts(&self) -> Option<&HashMap<String, Host>> {
        match self.hosts {
            Some(ref h) => Some(h),
            None => None,
        }
    }

    /// Add a `hosts` value.
    pub fn add_host(&mut self, k: &str, v: Host) -> &mut MusshToml {
        if let Some(ref mut hosts) = self.hosts {
            hosts.insert(k.to_string(), v);
        } else {
            let mut new_hosts = HashMap::new();
            new_hosts.insert(k.to_string(), v);
            self.hosts = Some(new_hosts);
        }
        self
    }

    /// Get the `cmd` value.
    pub fn cmd(&self) -> Option<&HashMap<String, Command>> {
        match self.cmd {
            Some(ref c) => Some(c),
            None => None,
        }
    }
}

impl Hosts {
    /// Get the `hostnames` value.
    pub fn hostnames(&self) -> &Vec<String> {
        &self.hostnames
    }
}

impl fmt::Display for Hosts {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let len = self.hostnames.len();
        for (idx, host) in self.hostnames.iter().enumerate() {
            write!(f, "{}", host)?;
            if idx < len - 1 {
                write!(f, ", ")?;
            }
        }
        Ok(())
    }
}

impl Host {
    /// Get the `hostname` value.
    pub fn hostname(&self) -> &str {
        &self.hostname
    }

    /// Set the `hostname` value.
    pub fn set_hostname(&mut self, hostname: &str) -> &mut Host {
        self.hostname = hostname.to_string();
        self
    }

    /// Get the `port` value.
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Set the `port` value.
    pub fn set_port(&mut self, port: u16) -> &mut Host {
        self.port = Some(port);
        self
    }

    /// Get the `username` value.
    pub fn username(&self) -> &str {
        &self.username
    }

    /// Set the `username` value.
    pub fn set_username(&mut self, username: &str) -> &mut Host {
        self.username = username.to_string();
        self
    }

    /// Get the `pem` value.
    pub fn pem(&self) -> Option<&str> {
        match self.pem {
            Some(ref p) => Some(p),
            None => None,
        }
    }

    /// Get the `alias` value.
    pub fn alias(&self) -> Option<HashMap<String, String>> {
        let mut aliases = HashMap::new();

        if let Some(ref alias_vec) = self.alias {
            for alias in alias_vec {
                aliases.insert(alias.aliasfor().to_string(), alias.command().to_string());
            }
        }

        if aliases.is_empty() {
            None
        } else {
            Some(aliases)
        }
    }
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut pem_str = String::new();
        if let Some(ref pem) = self.pem {
            pem_str.push(' ');
            pem_str.push_str(pem);
        }

        let mut aliases = String::new();
        if let Some(ref alias_vec) = self.alias {
            let len = alias_vec.len();
            for (idx, alias) in alias_vec.iter().enumerate() {
                if idx == 0 {
                    aliases.push_str(" { ");
                }
                if idx < len - 1 {
                    aliases.push_str(&format!("{}: {}, ", alias.aliasfor(), alias.command()));
                } else {
                    aliases.push_str(&format!("{}: {} }}", alias.aliasfor(), alias.command()));
                }
            }
        }

        write!(f,
               "{}@{}:{}{}{}",
               self.username,
               self.hostname,
               self.port.unwrap_or(22),
               pem_str,
               aliases)
    }
}

impl Command {
    /// Get the `command` value.
    pub fn command(&self) -> &str {
        &self.command
    }
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.command)
    }
}

impl Alias {
    /// Get the `command` value.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Get the `aliasfor` value.
    pub fn aliasfor(&self) -> &str {
        &self.aliasfor
    }
}

/// Generate a vector of paths to search for mussh.toml.
fn paths(arg: Option<PathBuf>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(dir) = arg {
        paths.push(dir);
    }

    if let Ok(mut cur_dir) = env::current_dir() {
        cur_dir.push(DOT_DIR);
        cur_dir.push(CONFIG_FILE_NAME);
        paths.push(cur_dir);
    }

    if let Some(mut home_dir) = env::home_dir() {
        home_dir.push(DOT_DIR);
        home_dir.push(CONFIG_FILE_NAME);
        paths.push(home_dir);
    }

    add_system_path(&mut paths);
    paths
}

#[cfg(windows)]
/// Add a system path to paths.
fn add_system_path(paths: &mut Vec<PathBuf>) {
    if let Ok(appdata) = env::var("APPDATA") {
        let mut appdata_path = PathBuf::from(appdata);
        appdata_path.push(DOT_DIR);
        appdata_path.push(CONFIG_FILE_NAME);
        paths.push(appdata_path);
    }
}

#[cfg(unix)]
/// Add a system path to paths.
fn add_system_path(paths: &mut Vec<PathBuf>) {
    let mut appdata = PathBuf::from("/etc");
    appdata.push("mussh");
    appdata.push(CONFIG_FILE_NAME);
    paths.push(appdata);
}


/// A `slog` drain that writes to a file.
#[derive(Debug)]
pub struct FileDrain {
    /// The file to drain log records to.
    file: File,
}

impl FileDrain {
    /// Create a new `FileDrain` that will write to a file at the given path.
    pub fn new(path: PathBuf) -> Result<FileDrain> {
        Ok(FileDrain {
               file: OpenOptions::new().create(true)
                   .append(true)
                   .open(path)?,
           })
    }
}

impl Drain for FileDrain {
    type Ok = ();
    type Err = Never;

    fn log(&self, record: &Record, _: &OwnedKVList) -> ::std::result::Result<(), Never> {
        if let Ok(mut log_file) = self.file.try_clone() {
            let utc: DateTime<UTC> = UTC::now();
            match writeln!(log_file, "{}: {}", utc.to_rfc3339(), record.msg()) {
                Ok(()) => {}
                Err(_e) => {}
            }
        }
        Ok(())
    }
}

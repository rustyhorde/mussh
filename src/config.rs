// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `mussh` config.
use clap::ArgMatches;
use error::{ErrorKind, Result};
use slog::{Drain, Level, LevelFilter, Logger};
use slog_async;
use slog_term;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use toml;

/// Default configuration filename.
pub const CONFIG_FILE_NAME: &'static str = "mussh.toml";
/// Default 'dot' directory for `mussh` configuration.
pub const DOT_DIR: &'static str = ".mussh";

pub struct Logging {
    /// The slog stdout `Logger`.
    stdout: Logger,
    /// The slog stderr `Logger`.
    #[allow(dead_code)]
    stderr: Logger,
}

impl Logging {
    /// Get the `stdout` value.
    pub fn stdout(&self) -> Logger {
        self.stdout.clone()
    }

    /// Set the stdout slog 'Logger' level.
    pub fn set_stdout_level(&mut self, level: Level) -> &mut Logging {
        self.stdout = stdout_logger(level);
        self
    }

    /// Get the `stderr` value.
    pub fn stderr(&self) -> Logger {
        self.stderr.clone()
    }
}

impl Default for Logging {
    fn default() -> Logging {
        Logging {
            stdout: stdout_logger(Level::Error),
            stderr: stderr_logger(),
        }
    }
}

/// Setup the stderr slog `Logger`
fn stderr_logger() -> Logger {
    let stderr_decorator = slog_term::TermDecorator::new().stderr().build();
    let stderr_drain = slog_term::CompactFormat::new(stderr_decorator).build().fuse();
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
    let stdout_drain = slog_term::CompactFormat::new(stdout_decorator).build().fuse();
    let stdout_async_drain = slog_async::Async::new(stdout_drain).build().fuse();
    let stdout_level_drain = LevelFilter::new(stdout_async_drain, level).fuse();
    Logger::root(stdout_level_drain,
                 o!(
        "executable" => env!("CARGO_PKG_NAME"),
        "version" => env!("CARGO_PKG_VERSION")
    ))
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
/// The base configuration.
pub struct MusshToml {
    /// A list of hosts.
    hostlist: Option<HashMap<String, Hosts>>,
    /// The hosts.
    hosts: Option<HashMap<String, Host>>,
    /// A command.
    cmd: Option<HashMap<String, Command>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
/// hosts configuration
pub struct Hosts {
    /// The hostnames.
    hostnames: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
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

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
/// command configuration
pub struct Command {
    /// A Command.
    command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
/// command alias configuration.
pub struct Alias {
    /// A command alias.
    command: String,
    /// The command this is an alias for.
    aliasfor: String,
}

impl MusshToml {
    /// Create a new 'MusshToml' from mussh.toml on the filesystem.
    pub fn new(matches: &ArgMatches) -> Result<MusshToml> {
        for path in &paths(matches.value_of("config")) {
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

impl Host {
    /// Get the `hostname` value.
    pub fn hostname(&self) -> &String {
        &self.hostname
    }

    /// Get the `port` value.
    pub fn port(&self) -> Option<u16> {
        self.port
    }

    /// Get the `username` value.
    pub fn username(&self) -> &String {
        &self.username
    }

    /// Get the `pem` value.
    pub fn pem(&self) -> Option<&String> {
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
                aliases.insert(alias.aliasfor().clone(), alias.command().clone());
            }
        }

        if aliases.is_empty() {
            None
        } else {
            Some(aliases)
        }
    }
}

impl Command {
    /// Get the `command` value.
    pub fn command(&self) -> &String {
        &self.command
    }
}

impl Alias {
    /// Get the `command` value.
    pub fn command(&self) -> &String {
        &self.command
    }

    /// Get the `aliasfor` value.
    pub fn aliasfor(&self) -> &String {
        &self.aliasfor
    }
}

/// Generate a vector of paths to search for mussh.toml.
fn paths(arg: Option<&str>) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(dir) = arg {
        paths.push(PathBuf::from(dir));
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

//! mussh configuration
use clap::ArgMatches;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use toml;

/// Default configuration filename.
pub const CONFIG_FILE_NAME: &'static str = "mussh.toml";
/// Default dot dir.
pub const DOT_DIR: &'static str = ".mussh";
/// Default stdout log file name.
pub const STDOUT_FILE: &'static str = "stdout.log";
/// Default stdout log file name.
pub const STDERR_FILE: &'static str = "stderr.log";

#[derive(Debug, Default, Deserialize)]
/// The base configuration.
pub struct MusshToml {
    /// A list of hosts.
    hostlist: Option<HashMap<String, Hosts>>,
    /// The hosts.
    hosts: Option<HashMap<String, Host>>,
    /// A command.
    cmd: Option<HashMap<String, Command>>,
}

#[derive(Debug, Default, Deserialize)]
/// hosts configuration
pub struct Hosts {
    /// The hostnames.
    hostnames: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
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
    alias: Vec<Alias>,
}

#[derive(Debug, Default, Deserialize)]
/// command configuration
pub struct Command {
    /// A Command.
    command: String,
}

#[derive(Debug, Default, Deserialize)]
/// command alias configuration.
pub struct Alias {
    /// A command alias.
    command: String,
    /// The command this is an alias for.
    aliasfor: String,
}

impl MusshToml {
    /// Create a new 'MusshToml' from mussh.toml on the filesystem.
    pub fn new(matches: &ArgMatches) -> MusshToml {
        let mut toml: MusshToml = Default::default();

        for path in &paths(matches.value_of("config")) {
            if let Ok(mut config_file) = File::open(path) {
                let mut toml_buf = vec![];
                if config_file.read_to_end(&mut toml_buf).is_ok() {
                    let toml_str = String::from_utf8_lossy(&toml_buf);
                    if let Ok(parsed) = toml::from_str(&toml_str) {
                        toml = parsed;
                        break;
                    }
                }
            }
        }

        toml
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
        for alias in &self.alias {
            aliases.insert(alias.aliasfor().clone(), alias.command().clone());
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
    appdata.push("goopd");
    appdata.push(CONFIG_FILE_NAME);
    paths.push(appdata);
}

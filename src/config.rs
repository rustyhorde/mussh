use failure::{Error, Fallible};
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

crate const MUSSH_CONFIG_FILE_NAME: &str = "mussh.toml";

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// The base configuration.
crate struct Mussh {
    /// A list of hosts.
    #[serde(serialize_with = "toml::ser::tables_last")]
    hostlist: BTreeMap<String, Hosts>,
    /// The hosts.
    #[serde(serialize_with = "toml::ser::tables_last")]
    hosts: BTreeMap<String, Host>,
    /// A command.
    #[serde(serialize_with = "toml::ser::tables_last")]
    cmd: BTreeMap<String, Command>,
}

impl TryFrom<PathBuf> for Mussh {
    type Error = Error;

    fn try_from(path: PathBuf) -> Fallible<Self> {
        let mut buf_reader = BufReader::new(File::open(path)?);
        let mut buffer = String::new();
        let _bytes_read = buf_reader.read_to_string(&mut buffer)?;
        Ok(toml::from_str(&buffer)?)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// hosts configuration
crate struct Hosts {
    /// The hostnames.
    hostnames: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// Host configuration.
crate struct Host {
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
crate struct Command {
    /// A Command.
    command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
/// command alias configuration.
crate struct Alias {
    /// A command alias.
    command: String,
    /// The command this is an alias for.
    aliasfor: String,
}

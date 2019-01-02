use failure::{Error, Fallible};
use getset::Getters;
use serde_derive::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
/// The base configuration.
crate struct Mussh {
    /// A list of hosts.
    #[serde(serialize_with = "toml::ser::tables_last")]
    #[get = "pub"]
    hostlist: BTreeMap<String, Hosts>,
    /// The hosts.
    #[serde(serialize_with = "toml::ser::tables_last")]
    #[get = "pub"]
    hosts: BTreeMap<String, Host>,
    /// A command.
    #[serde(serialize_with = "toml::ser::tables_last")]
    #[get = "pub"]
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

#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
/// hosts configuration
crate struct Hosts {
    /// The hostnames.
    #[get = "pub"]
    hostnames: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
/// Host configuration.
crate struct Host {
    /// A hostname.
    #[get = "pub"]
    hostname: String,
    /// A pem key.
    #[get = "pub"]
    pem: Option<String>,
    /// A port
    #[get = "pub"]
    port: Option<u16>,
    /// A username.
    #[get = "pub"]
    username: String,
    /// A command alias.
    #[get = "pub"]
    alias: Option<Vec<Alias>>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
/// command configuration
crate struct Command {
    /// A Command.
    #[get = "pub"]
    command: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, Getters, PartialEq, Serialize)]
/// command alias configuration.
crate struct Alias {
    /// A command alias.
    #[get = "pub"]
    command: String,
    /// The command this is an alias for.
    #[get = "pub"]
    aliasfor: String,
}

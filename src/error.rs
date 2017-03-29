// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! `mussh` errors
error_chain!{
    foreign_links {
        Ssh2(::ssh2::Error);
        Io(::std::io::Error);
        Term(::term::Error);
        TomlSe(::toml::ser::Error);
        ParseInt(::std::num::ParseIntError);
    }

    errors {
        Config {
            description("Could not find valid mussh.toml file!")
            display("Could not find valid mussh.toml file!")
        }
        InvalidCmd {
            description("'cmd' not configured properly in TOML!")
            display("'cmd' not configured properly in TOML!")
        }
        InvalidHostList {
            description("'hostlist' not configured properly in TOML!")
            display("'hostlist' not configured properly in TOML!")
        }
        InvalidHosts {
            description("'hosts' not configured properly in TOML!")
            display("'hosts' not configured properly in TOML!")
        }
        InvalidToml {
            description("Invalid TOML configuration!")
            display("Invalid TOML configuration!")
        }
        NoValidHosts {
            description("Could not determine any valid hosts!")
            display("Could not determine any valid hosts!")
        }
        HostNotConfigured(host: String) {
            description("host not configured!")
            display("host {} not configured!", host)
        }
        SshAuthentication {
            description("ssh authentication failed!")
            display("ssh authentication failed!")
        }
        SshSession {
            description("invalid ssh session!")
            display("invalid ssh session!")
        }
        SubCommand {
            description("invalid sub-command!")
            display("invalid sub-command!")
        }
        NoTerm {
            description("unable to get vallid term!")
            display("unable to get vallid term!")
        }
    }
}

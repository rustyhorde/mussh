// Copyright (c) 2016 mussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! runtime for `mussh`.
use clap::{App, Arg, SubCommand};
use cmd::{command, hostlist, hosts, run};
use config::Config;
use error::Result;
use slog::Level;

/// Run `mussh`
pub fn run() -> Result<i32> {
    let mut config: Config = Default::default();
    let stdout = config.stdout();
    let stderr = config.stderr();

    let matches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Jason Ozias <jason.g.ozias@gmail.com>")
        .about("ssh multiplexing client")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("CONFIG")
                .help("Specify a non-standard path for the TOML config file.")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Set the output verbosity level (more v's = more verbose)"),
        )
        .subcommand(
            SubCommand::with_name("cmd")
                .about("Work with 'cmd' configuration")
                .subcommand(SubCommand::with_name("list").about("List the 'cmd' configuration"))
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Add 'cmd' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'cmd' name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("cmd")
                                .value_name("CMD")
                                .help("The command string")
                                .index(2)
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("update")
                        .about("Update 'cmd' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'cmd' name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("cmd")
                                .value_name("CMD")
                                .help(
                                    "The command to update the \
                                     'cmd' with",
                                )
                                .index(2)
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Remove a 'cmd' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'cmd' name")
                                .index(1)
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("hostlist")
                .about("Work with 'hostlist' configuration")
                .subcommand(
                    SubCommand::with_name("list").about("List the 'hostlist' configuration"),
                )
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Add 'hostlist' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'hostlist' name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("hosts")
                                .value_name("HOSTS")
                                .multiple(true)
                                .help(
                                    "The hosts to add to the \
                                     'hostlist'",
                                )
                                .index(2)
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("update")
                        .about("Update 'hostlist' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'hosts' name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("hosts")
                                .value_name("HOSTS")
                                .multiple(true)
                                .help(
                                    "The hosts to upate the \
                                     'hostlist' with",
                                )
                                .index(2)
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Remove a 'hostlist' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'hostlist' name")
                                .index(1)
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("hosts")
                .about("Work with 'hosts' configuration")
                .subcommand(SubCommand::with_name("list").about("List the 'hosts' configuration"))
                .subcommand(
                    SubCommand::with_name("add")
                        .about("Add 'hosts' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'hosts' name")
                                .index(1)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("username")
                                .value_name("USERNAME")
                                .help("The username")
                                .index(2)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("hostname")
                                .value_name("HOSTNAME")
                                .help("The hostname/ip address")
                                .index(3)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("port")
                                .value_name("PORT")
                                .help("The port")
                                .index(4)
                                .required(true),
                        )
                        .arg(
                            Arg::with_name("pem")
                                .value_name("PEM")
                                .help("A pem file path")
                                .index(5),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("update")
                        .about("Update 'hosts' configuration")
                        .arg(
                            Arg::with_name("username")
                                .short("u")
                                .long("username")
                                .value_name("USERNAME")
                                .help("The updated username")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("hostname")
                                .short("h")
                                .long("hostname")
                                .value_name("hostname")
                                .help("The updated hostname")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("port")
                                .short("p")
                                .long("port")
                                .value_name("PORT")
                                .help("The updated port")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("pem")
                                .long("pem")
                                .value_name("PEM")
                                .help("The updated pem file path.")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'hosts' name")
                                .index(1)
                                .required(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("remove")
                        .about("Remove a 'hosts' configuration")
                        .arg(
                            Arg::with_name("name")
                                .value_name("NAME")
                                .help("The assigned 'hosts' name")
                                .index(1)
                                .required(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("run")
                .about("Run a command on hosts")
                .arg(Arg::with_name("dry_run").long("dryrun").help(
                    "Parse config and setup the client, \
                     but don't run it.",
                ))
                .arg(Arg::with_name("sync").short("s").long("sync").help(
                    "Run the given commadn synchronously across the \
                     hosts.",
                ))
                .arg(
                    Arg::with_name("command")
                        .value_name("CMD")
                        .help("The command to multiplex")
                        .index(1)
                        .required(true),
                )
                .arg(
                    Arg::with_name("hosts")
                        .value_name("HOSTS")
                        .multiple(true)
                        .help("The hosts to multiplex the command over")
                        .index(2)
                        .required(true),
                ),
        )
        .get_matches();

    // Setup the logging
    let level = match matches.occurrences_of("verbose") {
        0 => Level::Info,
        1 => Level::Debug,
        2 | _ => Level::Trace,
    };
    config.set_stdout_level(level);

    if let Some(toml_dir_string) = matches.value_of("config") {
        config.set_toml_dir(toml_dir_string);
    }

    match matches.subcommand() {
        // 'cmd' subcommand
        ("cmd", Some(sub_m)) => command::cmd(&mut config, sub_m, &stderr),
        // 'hostlist' subcommand
        ("hostlist", Some(sub_m)) => hostlist::cmd(&mut config, sub_m, &stderr),
        // 'hosts' subcommand
        ("hosts", Some(sub_m)) => hosts::cmd(&mut config, sub_m),
        // 'run' subcommand
        ("run", Some(sub_m)) => run::cmd(&mut config, sub_m, &stdout, &stderr),
        (cmd, _) => {
            error!(stderr, "Unknown subcommand {}", cmd);
            Ok(1)
        }
    }
}

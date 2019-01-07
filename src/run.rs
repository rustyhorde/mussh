// Copyright © 2016 libmussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Runtime
use crate::error::MusshResult;
use crate::logging::Loggers;
use crate::subcmd::{Run, Subcommand};
use clap::{App, Arg};
use libmussh::Config;
use slog::trace;
use slog_try::try_trace;
use std::convert::TryFrom;
use std::env;
use std::path::PathBuf;

crate const MUSSH_CONFIG_FILE_NAME: &str = "mussh.toml";

fn base_config_dir() -> MusshResult<PathBuf> {
    Ok(if let Some(config_dir) = dirs::config_dir() {
        config_dir
    } else if let Ok(current_dir) = env::current_dir() {
        current_dir
    } else {
        return Err("Unable to determine a suitable config directory!".into());
    }
    .join(env!("CARGO_PKG_NAME")))
}

crate fn run() -> MusshResult<()> {
    // Setup the default config path for use in clap App
    let base_path = base_config_dir()?;
    let base_path_str = format!("{}", base_path.display());
    let matches = app(&base_path_str).get_matches_safe()?;

    // Setup the slog Loggers
    let (stdout, stderr) = Loggers::try_from(&matches)?.split();

    // Grab the mussh config
    let config_path = PathBuf::from(matches.value_of("config").unwrap_or_else(|| "./"))
        .join(MUSSH_CONFIG_FILE_NAME);
    try_trace!(stdout, "Config Path: {}", config_path.display());
    let config = Config::try_from(config_path)?;

    if matches.is_present("output") {
        try_trace!(stdout, "{:?}", config);
    }

    // Run, run, run...
    match matches.subcommand() {
        // 'cmd' subcommand
        // ("cmd", Some(sub_m)) => command::cmd(&mut config, sub_m, &stderr),
        // 'hostlist' subcommand
        // ("hostlist", Some(sub_m)) => hostlist::cmd(&mut config, sub_m, &stderr),
        // 'hosts' subcommand
        // ("hosts", Some(sub_m)) => hosts::cmd(&mut config, sub_m),
        // 'run' subcommand
        ("run", Some(sub_m)) => Run::new(stdout, stderr).execute(&config, sub_m),
        (cmd, _) => Err(format!("Unknown subcommand {}", cmd).into()),
    }
}

fn app<'a, 'b>(default_config_path: &'a str) -> App<'a, 'b> {
    App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author("Jason Ozias <jason.g.ozias@gmail.com>")
        .about("ssh multiplexing client")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("CONFIG")
                .help("Specify a path for the TOML config file.")
                .default_value(default_config_path)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("dry_run")
                .short("d")
                .long("dry_run")
                .help("Load the configuration and display what would be run"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Set the output verbosity level (more v's = more verbose)"),
        )
        .arg(
            Arg::with_name("output")
                .short("o")
                .long("output")
                .help("Show the TOML configuration"),
        )
        .subcommand(Run::subcommand())
}

#[cfg(test)]
mod test {
    use super::app;
    use crate::error::MusshResult;
    use clap::ArgMatches;

    fn check_multiple_arg(m: &ArgMatches<'_>, name: &str, expected: &[&str]) {
        assert!(m.is_present(name));
        assert_eq!(m.occurrences_of(name), 1); // notice only one occurrence
        if let Some(values) = m.values_of(name) {
            let values_vec: Vec<_> = values.collect();
            assert_eq!(values_vec, expected);
        } else {
            assert!(false, "no values found!");
        }
    }

    #[test]
    fn full_run_subcmd() -> MusshResult<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh",
            "-vvv",
            "-c",
            "test_cfg",
            "--dry_run",
            "--output",
            "run",
            "-c",
            "python,nginx,tmux",
            "-h",
            "all,!m8",
            "--sync",
            "-s",
            "m4",
            "-y",
            "bar",
        ])?;

        if let ("run", Some(sub_m)) = app_m.subcommand() {
            // Check the commands
            check_multiple_arg(sub_m, "commands", &["python", "nginx", "tmux"]);
            // Check the hosts
            check_multiple_arg(sub_m, "hosts", &["all", "!m8"]);
            // Check for the presence of sync
            assert!(sub_m.is_present("sync"));
            // Check the group-cmds
            check_multiple_arg(sub_m, "sync_commands", &["bar"]);
            // Check the group-pre
            check_multiple_arg(sub_m, "sync_hosts", &["m4"]);
        } else {
            // Either no run subcommand or one not tested for...
            assert!(false, "Run subcommand not found!");
        }

        Ok(())
    }

    #[test]
    fn full_run_subcmd_alt_order_one() -> MusshResult<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh",
            "run",
            "-h",
            "all,!m8",
            "--sync",
            "-c",
            "python,nginx,tmux",
        ])?;

        if let ("run", Some(sub_m)) = app_m.subcommand() {
            // Check the commands
            check_multiple_arg(sub_m, "commands", &["python", "nginx", "tmux"]);
            // Check the hosts
            check_multiple_arg(sub_m, "hosts", &["all", "!m8"]);
            // Check for the presence of sync
            assert!(sub_m.is_present("sync"));
        } else {
            // Either no run subcommand or one not tested for...
            assert!(false, "Run subcommand not found!");
        }

        Ok(())
    }

    #[test]
    fn full_run_subcmd_alt_order_two() -> MusshResult<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh",
            "run",
            "--sync",
            "-h",
            "all,!m8",
            "-c",
            "python,nginx,tmux",
        ])?;

        if let ("run", Some(sub_m)) = app_m.subcommand() {
            // Check the commands
            check_multiple_arg(sub_m, "commands", &["python", "nginx", "tmux"]);
            // Check the hosts
            check_multiple_arg(sub_m, "hosts", &["all", "!m8"]);
            // Check for the presence of sync
            assert!(sub_m.is_present("sync"));
        } else {
            // Either no run subcommand or one not tested for...
            assert!(false, "Run subcommand not found!");
        }

        Ok(())
    }

    #[test]
    fn run_subcmd_no_sync() -> MusshResult<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh",
            "run",
            "-c",
            "python,nginx,tmux",
            "-h",
            "all,!m8",
        ])?;

        if let ("run", Some(sub_m)) = app_m.subcommand() {
            // Check the commands
            check_multiple_arg(sub_m, "commands", &["python", "nginx", "tmux"]);
            // Check the hosts
            check_multiple_arg(sub_m, "hosts", &["all", "!m8"]);
            // Check for the presence of sync
            assert!(!sub_m.is_present("sync"));
        } else {
            // Either no run subcommand or one not tested for...
            assert!(false, "Run subcommand not found!");
        }

        Ok(())
    }

    #[test]
    fn run_subcommand_missing_commands() {
        assert!(app("")
            .get_matches_from_safe(vec!["mussh", "run", "-h", "all", "!m8", "-s",])
            .is_err());
    }

    #[test]
    fn run_subcommand_missing_hosts() {
        assert!(app("")
            .get_matches_from_safe(vec!["mussh", "run", "-c", "python", "nginx", "tmux", "-s",])
            .is_err());
    }

    #[test]
    fn run_subcommand_missing_all() {
        assert!(app("").get_matches_from_safe(vec!["mussh", "run"]).is_err());
    }

    #[test]
    fn run_subcommand_missing_group() {
        assert!(app("")
            .get_matches_from_safe(vec![
                "mussh",
                "run",
                "--group-cmds",
                "bar",
                "--group-pre",
                "m4"
            ])
            .is_err());
    }

    #[test]
    fn run_subcommand_missing_group_pre() {
        assert!(app("")
            .get_matches_from_safe(vec![
                "mussh",
                "run",
                "--group-cmds",
                "bar",
                "--group",
                "m1,m2,m3"
            ])
            .is_err());
    }

    #[test]
    fn run_subcommand_missing_group_cmds() {
        assert!(app("")
            .get_matches_from_safe(vec![
                "mussh",
                "run",
                "--group-pre",
                "m4",
                "--group",
                "m1,m2,m3"
            ])
            .is_err());
    }
}

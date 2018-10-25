use clap::{App, Arg};
use crate::logging::{Loggers, Slogger};
use crate::subcmd::{Run, SubCmd};
use failure::Fallible;
use slog::trace;
use slog_try::try_trace;
use std::convert::TryFrom;
use std::env;
use std::path::PathBuf;

fn base_config_dir() -> Fallible<PathBuf> {
    Ok(if let Some(config_dir) = dirs::config_dir() {
        config_dir
    } else if let Ok(current_dir) = env::current_dir() {
        current_dir
    } else {
        return Err(failure::err_msg(
            "Unable to determine a suitable config directory!",
        ));
    }
    .join(env!("CARGO_PKG_NAME")))
}

crate fn run() -> Fallible<()> {
    let path_str = format!("{}", base_config_dir()?.display());
    let matches = app(&path_str).get_matches_safe()?;
    let (stdout, stderr) = Loggers::try_from(&matches)?.split();

    try_trace!(
        stdout,
        "Config Dir: {}",
        matches.value_of("config").unwrap_or_else(|| "")
    );

    match matches.subcommand() {
        // 'cmd' subcommand
        // ("cmd", Some(sub_m)) => command::cmd(&mut config, sub_m, &stderr),
        // 'hostlist' subcommand
        // ("hostlist", Some(sub_m)) => hostlist::cmd(&mut config, sub_m, &stderr),
        // 'hosts' subcommand
        // ("hosts", Some(sub_m)) => hosts::cmd(&mut config, sub_m),
        // 'run' subcommand
        ("run", Some(sub_m)) => Run::try_from(sub_m)?
            .set_stdout(stdout)
            .set_stderr(stderr)
            .cmd(),
        (cmd, _) => Err(failure::err_msg(format!("Unknown subcommand {}", cmd))),
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
            Arg::with_name("verbose")
                .short("v")
                .multiple(true)
                .help("Set the output verbosity level (more v's = more verbose)"),
        )
        .subcommand(Run::subcommand())
}

#[cfg(test)]
mod test {
    use super::app;
    use clap::ArgMatches;
    use failure::Fallible;

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
    fn full_run_subcmd() -> Fallible<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh", "-vvv", "run", "-c", "python", "nginx", "tmux", "-h", "all", "!m8", "-s",
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
    fn full_run_subcmd_alt_order_one() -> Fallible<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh", "run", "-h", "all", "!m8", "-s", "-c", "python", "nginx", "tmux",
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
    fn full_run_subcmd_alt_order_two() -> Fallible<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh", "run", "-s", "-h", "all", "!m8", "-c", "python", "nginx", "tmux",
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
    fn run_subcmd_no_sync() -> Fallible<()> {
        let app_m = app("").get_matches_from_safe(vec![
            "mussh", "run", "-c", "python", "nginx", "tmux", "-h", "all", "!m8",
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
}

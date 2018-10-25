use clap::{App, Arg};
use crate::subcmd::{Run, SubCmd};
use failure::Fallible;

crate fn run() -> Fallible<()> {
    match app().get_matches_safe()?.subcommand() {
        // 'cmd' subcommand
        // ("cmd", Some(sub_m)) => command::cmd(&mut config, sub_m, &stderr),
        // 'hostlist' subcommand
        // ("hostlist", Some(sub_m)) => hostlist::cmd(&mut config, sub_m, &stderr),
        // 'hosts' subcommand
        // ("hosts", Some(sub_m)) => hosts::cmd(&mut config, sub_m),
        // 'run' subcommand
        ("run", Some(sub_m)) => Run::cmd(sub_m),
        (cmd, _) => Err(failure::err_msg(format!("Unknown subcommand {}", cmd))),
    }
}

fn app<'a, 'b>() -> App<'a, 'b> {
    App::new(env!("CARGO_PKG_NAME"))
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
        let app_m = app().get_matches_from_safe(vec![
            "mussh", "run", "-c", "python", "nginx", "tmux", "-h", "all", "!m8", "-s",
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
        let app_m = app().get_matches_from_safe(vec![
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
        let app_m = app().get_matches_from_safe(vec![
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
        let app_m = app().get_matches_from_safe(vec![
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
        assert!(app()
            .get_matches_from_safe(vec!["mussh", "run", "-h", "all", "!m8", "-s",])
            .is_err());
    }

    #[test]
    fn run_subcommand_missing_hosts() {
        assert!(app()
            .get_matches_from_safe(vec!["mussh", "run", "-c", "python", "nginx", "tmux", "-s",])
            .is_err());
    }
}

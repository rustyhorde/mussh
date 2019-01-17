// Copyright © 2016 libmussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! run subcommand
use crate::error::MusshResult;
use crate::logging::FileDrain;
use crate::subcmd::Subcommand;
use clap::{App, Arg, ArgMatches, SubCommand};
use libmussh::{Config, Multiplex, RuntimeConfig};
use rusqlite::{Connection, NO_PARAMS};
use slog::{o, trace, Drain, Logger};
use slog_try::try_trace;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

#[derive(Clone, Default)]
crate struct Run {
    stdout: Option<Logger>,
    stderr: Option<Logger>,
    db_path: PathBuf,
}

impl Run {
    crate fn new(stdout: Option<Logger>, stderr: Option<Logger>, db_path: PathBuf) -> Self {
        Self {
            stdout,
            stderr,
            db_path,
        }
    }
}

impl Subcommand for Run {
    fn subcommand<'a, 'b>() -> App<'a, 'b> {
        SubCommand::with_name("run")
            .about("Run a command on hosts")
            .arg(Arg::with_name("dry_run").long("dryrun").help(
                "Parse config and setup the client, \
                 but don't run it.",
            ))
            .arg(
                Arg::with_name("hosts")
                    .short("h")
                    .long("hosts")
                    .value_name("HOSTS")
                    .help("The hosts to multiplex the command over")
                    .multiple(true)
                    .use_delimiter(true),
            )
            .arg(
                Arg::with_name("commands")
                    .short("c")
                    .long("commands")
                    .value_name("CMD")
                    .help("The commands to multiplex")
                    .multiple(true)
                    .requires("hosts")
                    .use_delimiter(true),
            )
            .arg(
                Arg::with_name("sync_hosts")
                    .short("s")
                    .long("sync_hosts")
                    .value_name("HOSTS")
                    .help("The hosts to run the sync commands on before running on any other hosts")
                    .use_delimiter(true)
                    .required_unless("hosts")
                    .requires("sync_commands"),
            )
            .arg(
                Arg::with_name("sync_commands")
                    .short("y")
                    .long("sync_commands")
                    .value_name("CMD")
                    .help("The commands to run on the sync hosts before running on any other hosts")
                    .use_delimiter(true),
            )
            .arg(Arg::with_name("sync").long("sync").help(
                "Run the given commadn synchronously across the \
                 hosts.",
            ))
    }

    fn execute(&self, config: &Config, matches: &ArgMatches<'_>) -> MusshResult<()> {
        let runtime_config = RuntimeConfig::from(matches);
        let sync_hosts = runtime_config.sync_hosts();
        let multiplex_map = config.to_host_map(&runtime_config);
        let conn = Connection::open(&self.db_path)?;
        create_metrics_table(&conn)?;

        let mut cmd_loggers_map = HashMap::new();
        for host in multiplex_map.keys() {
            let _ = cmd_loggers_map
                .entry(host.clone())
                .or_insert_with(|| host_file_logger(&self.stdout, host));
        }
        let mut multiplex = Multiplex::default();
        let _ = multiplex.set_stdout(self.stdout.clone());
        let _ = multiplex.set_stderr(self.stderr.clone());
        let _ = multiplex.set_host_loggers(cmd_loggers_map);
        for result in multiplex.multiplex(sync_hosts, multiplex_map) {
            if let Ok(metrics) = result {
                let secs = metrics.duration().as_secs();
                let ms = metrics.duration().subsec_millis();
                println!(
                    "'{}' run on '{}' in {}.{}",
                    metrics.cmd_name(),
                    metrics.hostname(),
                    secs,
                    ms
                );
            }
        }

        Ok(())
    }
}

fn create_metrics_table(conn: &Connection) -> MusshResult<()> {
    let _rows_changed = conn.execute(
        "CREATE TABLE IF NOT EXISTS metrics (
          id         INTEGER PRIMARY KEY,
          hostname   TEXT NOT NULL,
          cmdname    TEXT NOT NULL,
          secs       INTEGER NOT NULL,
          micros     INTEGER NOT NULL,
          timestamp  INTEGER NOT NULL
        )",
        NO_PARAMS,
    )?;
    Ok(())
}

fn host_file_logger(stdout: &Option<Logger>, hostname: &str) -> Option<Logger> {
    let mut host_file_path = if let Some(mut config_dir) = dirs::config_dir() {
        config_dir.push(env!("CARGO_PKG_NAME"));
        config_dir
    } else {
        PathBuf::new()
    };

    host_file_path.push(hostname);
    let _ = host_file_path.set_extension("log");

    try_trace!(stdout, "Log Path: {}", host_file_path.display());

    if let Ok(file_drain) = FileDrain::try_from(host_file_path) {
        let async_file_drain = slog_async::Async::new(file_drain).build().fuse();
        let file_logger = Logger::root(async_file_drain, o!());
        Some(file_logger)
    } else {
        None
    }
}

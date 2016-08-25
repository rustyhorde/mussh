use {MusshResult, STDERR_SW, STDOUT_SW};
use clap::{App, Arg};
use slog::Level;
use slog::drain::{self, IntoLogger};
use slog_json;
use slog_term;
use std::fs::OpenOptions;
use std::io;

fn multiplex() -> MusshResult<()> {
    Ok(())
}

pub fn run(opt_args: Option<Vec<&str>>) -> i32 {
    let app = App::new("mussh")
        .version(crate_version!())
        .author("Jason Ozias <jason.g.ozias@gmail.com>")
        .about("ssh multiplexing client")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("CONFIG")
            .help("Specify a non-standard path for the config file.")
            .takes_value(true))
        .arg(Arg::with_name("dry_run")
            .long("dryrun")
            .help("Parse config and setup the client, but don't run it."))
        .arg(Arg::with_name("verbose")
            .short("v")
            .multiple(true)
            .help("Set the output verbosity level (more v's = more verbose)"))
        .arg(Arg::with_name("json")
            .short("j")
            .long("json")
            .help("Enable json logging at the given path")
            .value_name("PATH")
            .takes_value(true));

    let matches = if let Some(args) = opt_args {
        app.get_matches_from(args)
    } else {
        app.get_matches()
    };

    // Setup the logging
    let level = match matches.occurrences_of("verbose") {
        0 => Level::Error,
        1 => Level::Warning,
        2 => Level::Info,
        3 => Level::Debug,
        4 | _ => Level::Trace,
    };

    let mut json_drain = None;
    if let Some(json_path) = matches.value_of("json") {
        if let Ok(json_file) = OpenOptions::new().create(true).append(true).open(json_path) {
            json_drain = Some(drain::stream(json_file, slog_json::new()));
        }
    }

    let stdout_base = drain::async_stream(io::stdout(), slog_term::format_colored());
    if let Some(json) = json_drain {
        STDOUT_SW.set(drain::filter_level(level, drain::duplicate(stdout_base, json)));
    } else {
        STDOUT_SW.set(drain::filter_level(level, stdout_base));
    }

    if matches.is_present("dry_run") {
        let stdout = STDOUT_SW.drain().into_logger(o!());
        warn!(stdout, "run", "message" => "Not starting event loop!", "dryrun" => "true");
        0
    } else if let Err(e) = multiplex() {
        let stderr = STDERR_SW.drain().into_logger(o!());
        error!(stderr, "run", "error" => "error running event_loops", "detail" => format!("{}", e));
        1
    }
    else {
        0
    }
}

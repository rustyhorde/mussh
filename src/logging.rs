//! Logging for the server.
use clap::ArgMatches;
use failure::Error;
use getset::Getters;
use slog::{o, Drain, Level, Logger};
use slog_async::Async;
use slog_term::{CompactFormat, TermDecorator};
use std::convert::TryFrom;

/// A struct that supports slog logging
crate trait Slogger {
    /// Add an optional stdout `slog` logger to the struct.
    fn set_stdout(self, stdout: Option<Logger>) -> Self;
    /// Add an optional stderr `slog` logger to the struct.
    fn set_stderr(self, stderr: Option<Logger>) -> Self;
}

/// `slog` loggers for stdout/stderr.
#[derive(Clone, Debug, Default, Getters)]
crate struct Loggers {
    /// An optional stdout logger.
    #[get = "pub"]
    stdout: Option<Logger>,
    /// An optional stderr logger.
    #[get = "pub"]
    stderr: Option<Logger>,
}

impl Loggers {
    /// Split this `Loggers` into the stdout and stderr components.
    crate fn split(&self) -> (Option<Logger>, Option<Logger>) {
        (self.stdout.clone(), self.stderr.clone())
    }
}

impl<'a> TryFrom<&'a ArgMatches<'a>> for Loggers {
    type Error = Error;

    fn try_from(matches: &'a ArgMatches<'a>) -> Result<Self, Error> {
        let level = match matches.occurrences_of("verbose") {
            0 => Level::Warning,
            1 => Level::Info,
            2 => Level::Debug,
            3 | _ => Level::Trace,
        };

        let stdout_decorator = TermDecorator::new().stdout().build();
        let stdout_drain = CompactFormat::new(stdout_decorator).build().fuse();
        let stdout_async_drain = Async::new(stdout_drain).build().filter_level(level).fuse();
        let stdout = Logger::root(stdout_async_drain, o!());

        let stderr_decorator = TermDecorator::new().stdout().build();
        let stderr_drain = CompactFormat::new(stderr_decorator).build().fuse();
        let stderr_async_drain = Async::new(stderr_drain)
            .build()
            .filter_level(Level::Error)
            .fuse();
        let stderr = Logger::root(stderr_async_drain, o!());

        Ok(Self {
            stdout: Some(stdout),
            stderr: Some(stderr),
        })
    }
}

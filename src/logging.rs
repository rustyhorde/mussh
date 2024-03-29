// Copyright © 2016 libmussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Logging for the server.
use crate::error::{MusshErr, MusshResult};
use chrono::{DateTime, Utc};
use clap::ArgMatches;
use getset::Getters;
use slog::{o, Drain, Level, Logger, Never, OwnedKVList, Record};
use slog_async::Async;
use slog_term::{CompactFormat, TermDecorator};
use std::convert::TryFrom;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// A struct that supports slog logging
pub(crate) trait Slogger {
    /// Add an optional stdout `slog` logger to the struct.
    fn set_stdout(self, stdout: Option<Logger>) -> Self;
    /// Add an optional stderr `slog` logger to the struct.
    fn set_stderr(self, stderr: Option<Logger>) -> Self;
}

/// `slog` loggers for stdout/stderr.
#[derive(Clone, Debug, Default, Getters)]
pub(crate) struct Loggers {
    /// An optional stdout logger.
    #[get = "pub"]
    stdout: Option<Logger>,
    /// An optional stderr logger.
    #[get = "pub"]
    stderr: Option<Logger>,
}

impl Loggers {
    /// Split this `Loggers` into the stdout and stderr components.
    pub(crate) fn split(&self) -> (Option<Logger>, Option<Logger>) {
        (self.stdout.clone(), self.stderr.clone())
    }
}

impl<'a> TryFrom<&'a ArgMatches<'a>> for Loggers {
    type Error = MusshErr;

    fn try_from(matches: &'a ArgMatches<'a>) -> Result<Self, MusshErr> {
        let level = match matches.occurrences_of("verbose") {
            0 => Level::Warning,
            1 => Level::Info,
            2 => Level::Debug,
            _ => Level::Trace,
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

/// A `slog` drain that writes to a file.
#[derive(Debug)]
#[allow(dead_code)]
pub(crate) struct FileDrain {
    /// The file to drain log records to.
    file: File,
}

impl TryFrom<PathBuf> for FileDrain {
    type Error = MusshErr;
    fn try_from(path: PathBuf) -> MusshResult<Self> {
        Ok(Self {
            file: OpenOptions::new().create(true).append(true).open(path)?,
        })
    }
}

impl Drain for FileDrain {
    type Ok = ();
    type Err = Never;

    fn log(&self, record: &Record<'_>, _: &OwnedKVList) -> ::std::result::Result<(), Never> {
        if let Ok(mut log_file) = self.file.try_clone() {
            let utc: DateTime<Utc> = Utc::now();
            match writeln!(log_file, "{}: {}", utc.to_rfc3339(), record.msg()) {
                Ok(()) => {}
                Err(_e) => {}
            }
        }
        Ok(())
    }
}

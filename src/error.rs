// Copyright © 2016 libmussh developers
//
// Licensed under the Apache License, Version 2.0
// <LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0> or the MIT
// license <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. All files in the project carrying such notice may not be copied,
// modified, or distributed except according to those terms.

//! Error Handling
use std::error::Error;
use std::fmt;

/// A result that includes a `mussh::Error`
crate type MusshResult<T> = Result<T, MusshErr>;

/// An error thrown by the mussh library
#[derive(Debug)]
crate struct MusshErr {
    /// The kind of error
    inner: MusshErrKind,
}

impl Error for MusshErr {
    fn description(&self) -> &str {
        "Mussh Error"
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}

impl fmt::Display for MusshErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let err: &(dyn Error) = self;
        let mut iter = err.chain();
        let _skip_me = iter.next();
        write!(f, "{}", self.inner)?;

        for e in iter {
            writeln!(f)?;
            write!(f, "{}", e)?;
        }
        Ok(())
    }
}

macro_rules! external_error {
    ($error:ty, $kind:expr) => {
        impl From<$error> for MusshErr {
            fn from(inner: $error) -> Self {
                Self {
                    inner: $kind(inner),
                }
            }
        }
    };
}

impl From<MusshErrKind> for MusshErr {
    fn from(inner: MusshErrKind) -> Self {
        Self { inner }
    }
}

impl From<&str> for MusshErr {
    fn from(inner: &str) -> Self {
        Self {
            inner: MusshErrKind::Str(inner.to_string()),
        }
    }
}

external_error!(clap::Error, MusshErrKind::Clap);
external_error!(std::io::Error, MusshErrKind::Io);
external_error!(libmussh::Error, MusshErrKind::Libmussh);
external_error!(String, MusshErrKind::Str);
external_error!(rusqlite::Error, MusshErrKind::Rusqlite);

#[derive(Debug)]
crate enum MusshErrKind {
    Clap(clap::Error),
    Io(std::io::Error),
    Libmussh(libmussh::Error),
    Rusqlite(rusqlite::Error),
    Str(String),
}

impl Error for MusshErrKind {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MusshErrKind::Clap(inner) => inner.source(),
            MusshErrKind::Io(inner) => inner.source(),
            MusshErrKind::Libmussh(inner) => inner.source(),
            MusshErrKind::Rusqlite(inner) => inner.source(),
            _ => None,
        }
    }
}

impl fmt::Display for MusshErrKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self)
    }
}

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
        write!(f, "{}", self.description())?;

        if let Some(source) = self.source() {
            write!(f, ": {}", source)?;
        }
        write!(f, "")
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

#[derive(Debug)]
crate enum MusshErrKind {
    Clap(clap::Error),
    Io(std::io::Error),
    Libmussh(libmussh::Error),
    Str(String),
}

impl Error for MusshErrKind {
    fn description(&self) -> &str {
        match self {
            MusshErrKind::Clap(inner) => inner.description(),
            MusshErrKind::Io(inner) => inner.description(),
            MusshErrKind::Libmussh(inner) => inner.description(),
            MusshErrKind::Str(inner) => &inner[..],
        }
    }

    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            MusshErrKind::Clap(inner) => inner.source(),
            MusshErrKind::Io(inner) => inner.source(),
            MusshErrKind::Libmussh(inner) => inner.source(),
            _ => None,
        }
    }
}

impl fmt::Display for MusshErrKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

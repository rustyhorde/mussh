use failure::{Backtrace, Context, Fail};
use std::fmt::{self, Display};

#[derive(Debug)]
crate struct MusshError {
    inner: Context<MusshErrorKind>,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
crate enum MusshErrorKind {
    #[fail(display = "The TOML configuration is invalid")]
    InvalidConfigToml,
    #[fail(display = "Failed to establish SSH session")]
    SshSession,
    #[fail(display = "Failed to authenticate for SSH session")]
    SshAuthentication,
    #[fail(display = "Failed to find a carshell to execute locally")]
    ShellNotFound,
}

impl Fail for MusshError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl Display for MusshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.inner, f)
    }
}

impl MusshError {
    #[allow(dead_code)]
    crate fn kind(&self) -> MusshErrorKind {
        *self.inner.get_context()
    }
}

impl From<MusshErrorKind> for MusshError {
    fn from(kind: MusshErrorKind) -> Self {
        Self {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<MusshErrorKind>> for MusshError {
    fn from(inner: Context<MusshErrorKind>) -> Self {
        Self { inner: inner }
    }
}

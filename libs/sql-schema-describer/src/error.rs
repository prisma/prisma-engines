#![deny(missing_docs)]

use std::{
    error::Error,
    fmt::{self, Display},
};
use tracing_error::SpanTrace;

/// The result type.
pub type DescriberResult<T> = Result<T, DescriberError>;

/// Description errors.
#[derive(Debug)]
pub struct DescriberError {
    kind: DescriberErrorKind,
    context: SpanTrace,
}

impl DescriberError {
    /// The `DescriberErrorKind` wrapped by the error.
    pub fn into_kind(self) -> DescriberErrorKind {
        self.kind
    }

    /// The `tracing_error::SpanTrace` contained in the error.
    pub fn span_trace(&self) -> SpanTrace {
        self.context.clone()
    }
}

/// Variants of DescriberError.
#[derive(Debug)]
pub enum DescriberErrorKind {
    /// An error originating from Quaint or the database.
    QuaintError(quaint::error::Error),
}

impl Display for DescriberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            DescriberErrorKind::QuaintError(err) => {
                err.fmt(f)?;
                self.context.fmt(f)
            }
        }
    }
}

impl Error for DescriberError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            DescriberErrorKind::QuaintError(err) => Some(err),
        }
    }
}

impl From<quaint::error::Error> for DescriberError {
    fn from(err: quaint::error::Error) -> Self {
        DescriberError {
            kind: DescriberErrorKind::QuaintError(err),
            context: SpanTrace::capture(),
        }
    }
}

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

    /// The `DescriberErrorKind` wrapped by the error.
    pub fn kind(&self) -> &DescriberErrorKind {
        &self.kind
    }

    /// The `tracing_error::SpanTrace` contained in the error.
    pub fn span_trace(&self) -> SpanTrace {
        self.context.clone()
    }
}

impl From<DescriberErrorKind> for DescriberError {
    fn from(kind: DescriberErrorKind) -> Self {
        Self {
            kind,
            context: SpanTrace::capture(),
        }
    }
}

/// Variants of DescriberError.
#[derive(Debug)]
pub enum DescriberErrorKind {
    /// An error originating from Quaint or the database.
    QuaintError(quaint::error::Error),
    /// An illegal cross-schema reference.
    CrossSchemaReference {
        /// Qualified path of the source table.
        from: String,
        /// Qualified path of the referenced table.
        to: String,
        /// Name of the constraint.
        constraint: String,
        /// This must be added to the schemas property.
        missing_namespace: String,
    },
}

impl Display for DescriberError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            DescriberErrorKind::QuaintError(_) => {
                self.kind().fmt(f)?;
                self.context.fmt(f)
            }
            _ => self.kind().fmt(f),
        }
    }
}

impl Display for DescriberErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QuaintError(err) => err.fmt(f),
            Self::CrossSchemaReference {
                from,
                to,
                constraint,
                missing_namespace,
            } => {
                write!(
                    f,
                    "The schema of the introspected database was inconsistent: Cross schema references are only allowed when the target schema is listed in the schemas property of your datasource. `{from}` points to `{to}` in constraint `{constraint}`. Please add `{missing_namespace}` to your `schemas` property and run this command again.",
                )
            }
        }
    }
}

impl Error for DescriberError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            DescriberErrorKind::QuaintError(err) => Some(err),
            DescriberErrorKind::CrossSchemaReference { .. } => None,
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

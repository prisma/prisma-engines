//! The migration connector ConnectorError type.

use crate::migrations_directory::ReadMigrationScriptError;
use std::{error::Error as StdError, fmt::Display};
use tracing_error::SpanTrace;
use user_facing_errors::{KnownError, UserFacingError};

/// The general error reporting type for migration connectors.
#[derive(Debug)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    report: anyhow::Error,
    /// See the tracing-error docs.
    context: SpanTrace,
}

impl Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}\n{}", self.report, self.context)
    }
}

impl StdError for ConnectorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.report.as_ref())
    }
}

impl ConnectorError {
    /// A reference to the tracing-error context.
    pub fn context(&self) -> &SpanTrace {
        &self.context
    }

    /// The user-facing error code for this error.
    pub fn error_code(&self) -> Option<&'static str> {
        self.user_facing_error.as_ref().map(|err| err.error_code)
    }

    /// Construct a `Generic` connector error.
    pub fn generic(report: anyhow::Error) -> Self {
        ConnectorError {
            user_facing_error: None,
            report,
            context: SpanTrace::capture(),
        }
    }

    /// Turn the error into a nested, user-facing MigrationDoesNotApplyCleanly error.
    pub fn into_migration_does_not_apply_cleanly(self, migration_name: String) -> Self {
        let context = self.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly {
            migration_name,
            inner_error: self
                .user_facing_error
                .clone()
                .map(user_facing_errors::Error::new_known)
                .unwrap_or_else(|| user_facing_errors::Error::new_non_panic_with_current_backtrace(self.to_string())),
        };

        ConnectorError {
            user_facing_error: Some(KnownError::new(user_facing_error)),
            report: self.into(),
            context,
        }
    }

    /// Access the inner `user_facing_error::KnownError`.
    pub fn known_error(&self) -> Option<&KnownError> {
        self.user_facing_error.as_ref()
    }

    /// Render to a user_facing_error::Error
    pub fn to_user_facing(&self) -> user_facing_errors::Error {
        match &self.user_facing_error {
            Some(known_error) => known_error.clone().into(),
            None => user_facing_errors::Error::from_dyn_error(self),
        }
    }

    /// Construct a GenericError with an associated user facing error.
    pub fn user_facing_error<T: UserFacingError>(err: T) -> Self {
        let report = anyhow::anyhow!("{}", err.message());
        ConnectorError {
            user_facing_error: Some(KnownError::new(err)),
            report,
            context: SpanTrace::capture(),
        }
    }

    /// Construct an UrlParseError.
    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        Self::generic(anyhow::anyhow!("{} in `{}`", err, url))
    }
}

impl From<KnownError> for ConnectorError {
    fn from(err: KnownError) -> Self {
        let report = anyhow::anyhow!("{}", err.message);

        ConnectorError {
            user_facing_error: Some(err),
            report,
            context: SpanTrace::capture(),
        }
    }
}

impl From<ReadMigrationScriptError> for ConnectorError {
    fn from(err: ReadMigrationScriptError) -> Self {
        let context = err.1.clone();

        ConnectorError {
            user_facing_error: None,
            report: err.into(),
            context,
        }
    }
}

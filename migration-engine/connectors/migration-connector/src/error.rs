//! The migration connector ConnectorError type.

use crate::migrations_directory::ReadMigrationScriptError;
use std::{error::Error as StdError, fmt::Display};
use tracing_error::SpanTrace;
use user_facing_errors::{migration_engine::MigrationFileNotFound, KnownError, UserFacingError};

/// The general error reporting type for migration connectors.
#[derive(Debug)]
pub struct ConnectorError(Box<ConnectorErrorImpl>);

#[derive(Debug)]
struct ConnectorErrorImpl {
    /// An optional error already rendered for users in case the migration core does not handle it.
    user_facing_error: Option<KnownError>,
    /// The error information for internal use.
    report: anyhow::Error,
    /// See the tracing-error docs.
    context: SpanTrace,
}

impl Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:#}\n{}", self.0.report, self.0.context)
    }
}

impl StdError for ConnectorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.0.report.as_ref())
    }
}

impl ConnectorError {
    /// A reference to the tracing-error context.
    pub fn context(&self) -> &SpanTrace {
        &self.0.context
    }

    /// The user-facing error code for this error.
    pub fn error_code(&self) -> Option<&'static str> {
        self.0.user_facing_error.as_ref().map(|err| err.error_code)
    }

    /// Construct a `Generic` connector error.
    pub fn generic(report: anyhow::Error) -> Self {
        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: None,
            report,
            context: SpanTrace::capture(),
        }))
    }

    /// Turn the error into a nested, user-facing MigrationDoesNotApplyCleanly error.
    pub fn into_migration_does_not_apply_cleanly(self, migration_name: String) -> Self {
        let context = self.0.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly {
            migration_name,
            inner_error: self.to_user_facing(),
        };

        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: Some(KnownError::new(user_facing_error)),
            report: self.into(),
            context,
        }))
    }

    /// Turn the error into a nested, user-facing ShadowDbCreationError.
    pub fn into_shadow_db_creation_error(self) -> Self {
        let context = self.0.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::ShadowDbCreationError {
            inner_error: self.to_user_facing(),
        };

        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: Some(KnownError::new(user_facing_error)),
            report: self.into(),
            context,
        }))
    }

    /// Turn the error into a nested, user-facing SoftResetFailed error.
    pub fn into_soft_reset_failed_error(self) -> Self {
        let context = self.0.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::SoftResetFailed {
            inner_error: self.to_user_facing(),
        };

        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: Some(KnownError::new(user_facing_error)),
            report: self.into(),
            context,
        }))
    }

    /// Access the inner `user_facing_error::KnownError`.
    pub fn known_error(&self) -> Option<&KnownError> {
        self.0.user_facing_error.as_ref()
    }

    /// Render to a user_facing_error::Error
    pub fn to_user_facing(&self) -> user_facing_errors::Error {
        match &self.0.user_facing_error {
            Some(known_error) => known_error.clone().into(),
            None => user_facing_errors::Error::from_dyn_error(self),
        }
    }

    /// Construct a GenericError with an associated user facing error.
    pub fn user_facing_error<T: UserFacingError>(err: T) -> Self {
        let report = anyhow::anyhow!("{}", err.message());

        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: Some(KnownError::new(err)),
            report,
            context: SpanTrace::capture(),
        }))
    }

    /// Construct an UrlParseError.
    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        Self::generic(anyhow::anyhow!("{} in `{}`", err, url))
    }
}

impl From<KnownError> for ConnectorError {
    fn from(err: KnownError) -> Self {
        let report = anyhow::anyhow!("{}", err.message);

        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: Some(err),
            report,
            context: SpanTrace::capture(),
        }))
    }
}

impl From<ReadMigrationScriptError> for ConnectorError {
    fn from(err: ReadMigrationScriptError) -> Self {
        let context = err.1.clone();

        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: Some(KnownError::new(MigrationFileNotFound {
                migration_file_path: err.2.clone(),
            })),
            report: err.into(),
            context,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::ConnectorError;

    #[test]
    fn connector_error_has_the_expected_size() {
        assert_eq!(std::mem::size_of::<ConnectorError>(), std::mem::size_of::<*mut ()>());
    }
}

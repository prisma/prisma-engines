//! The migration connector ConnectorError type.

use crate::{migrations_directory::ReadMigrationScriptError, ListMigrationsError};
use std::{error::Error as StdError, fmt::Display};
use tracing_error::SpanTrace;
use user_facing_errors::{migration_engine::MigrationFileNotFound, KnownError, UserFacingError};

/// The general error reporting type for migration connectors.
#[derive(Debug)]
pub struct ConnectorError {
    /// An optional error already rendered for users in case the migration core does not handle it.
    user_facing_error: Option<Box<KnownError>>,
    /// The error to be displayed. `Result` here is meant as an `Either` type:
    /// either an error we introduced, or something propagated from a previous error.
    subject: Result<Box<str>, Box<dyn StdError + Sync + Send + 'static>>,
    /// See the tracing-error docs.
    context: SpanTrace,
}

impl Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.subject {
            Ok(message) => f.write_str(&message)?,
            Err(source) => Display::fmt(&source, f)?,
        };

        Display::fmt(&self.context, f)
    }
}

impl StdError for ConnectorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self.subject.as_ref().err() {
            Some(err) => Some(err.as_ref()),
            None => None,
        }
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

    /// Build an unknown connector error from an error message.
    pub fn from_message(message: String) -> Self {
        ConnectorError {
            user_facing_error: None,
            subject: Ok(message.into_boxed_str()),
            context: SpanTrace::capture(),
        }
    }

    /// Turn the error into a nested, user-facing MigrationDoesNotApplyCleanly error.
    pub fn into_migration_does_not_apply_cleanly(self, migration_name: String) -> Self {
        let context = self.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly {
            migration_name,
            inner_error: self.to_user_facing(),
        };

        ConnectorError {
            user_facing_error: Some(Box::new(KnownError::new(user_facing_error))),
            subject: Err(Box::new(self)),
            context,
        }
    }

    /// Turn the error into a nested, user-facing ShadowDbCreationError.
    pub fn into_shadow_db_creation_error(self) -> Self {
        let context = self.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::ShadowDbCreationError {
            inner_error: self.to_user_facing(),
        };

        ConnectorError {
            user_facing_error: Some(Box::new(KnownError::new(user_facing_error))),
            subject: Err(Box::new(self)),
            context,
        }
    }

    /// Turn the error into a nested, user-facing SoftResetFailed error.
    pub fn into_soft_reset_failed_error(self) -> Self {
        let context = self.context.clone();
        let user_facing_error = user_facing_errors::migration_engine::SoftResetFailed {
            inner_error: self.to_user_facing(),
        };

        ConnectorError {
            user_facing_error: Some(Box::new(KnownError::new(user_facing_error))),
            subject: Err(Box::new(self)),
            context,
        }
    }

    /// Access the inner `user_facing_error::KnownError`.
    pub fn known_error(&self) -> Option<&KnownError> {
        self.user_facing_error.as_deref()
    }

    /// Build an unknown connector error from an unspecified downstream error.
    /// This should only be used when we can't do better and just want to bubble
    /// up the error with some context.
    pub fn propagate(source: Box<dyn StdError + Sync + Send + 'static>) -> Self {
        ConnectorError {
            user_facing_error: None,
            subject: Err(source),
            context: SpanTrace::capture(),
        }
    }

    /// Render to a user_facing_error::Error
    pub fn to_user_facing(&self) -> user_facing_errors::Error {
        match &self.user_facing_error {
            Some(known_error) => (**known_error).clone().into(),
            None => user_facing_errors::Error::from_dyn_error(self),
        }
    }

    /// Construct a GenericError with an associated user facing error.
    pub fn user_facing_error<T: UserFacingError>(err: T) -> Self {
        ConnectorError {
            subject: Ok(err.message().into_boxed_str()),
            user_facing_error: Some(Box::new(KnownError::new(err))),
            context: SpanTrace::capture(),
        }
    }

    /// Construct an UrlParseError.
    pub fn url_parse_error(err: impl Display, url: &str) -> Self {
        Self::from_message(format!("{} in `{}`", err, url))
    }
}

impl From<KnownError> for ConnectorError {
    fn from(err: KnownError) -> Self {
        ConnectorError {
            subject: Ok(err.message.clone().into_boxed_str()),
            user_facing_error: Some(Box::new(err)),
            context: SpanTrace::capture(),
        }
    }
}

impl From<ReadMigrationScriptError> for ConnectorError {
    fn from(err: ReadMigrationScriptError) -> Self {
        let context = err.1.clone();

        ConnectorError {
            user_facing_error: Some(Box::new(KnownError::new(MigrationFileNotFound {
                migration_file_path: err.2.clone(),
            }))),
            subject: Err(Box::new(err)),
            context,
        }
    }
}

impl From<ListMigrationsError> for ConnectorError {
    fn from(err: ListMigrationsError) -> Self {
        ConnectorError::propagate(Box::new(err))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_error_has_the_right_size() {
        assert_eq!(std::mem::size_of::<SpanTrace>(), 32);
        assert_eq!(std::mem::size_of::<Option<Box<KnownError>>>(), 8);
        assert_eq!(std::mem::size_of::<ConnectorError>(), 64);
    }
}

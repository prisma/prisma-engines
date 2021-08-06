//! The migration connector ConnectorError type.

use crate::migrations_directory::{ListMigrationsError, ReadMigrationScriptError};
use std::{
    error::Error as StdError,
    fmt::{Debug, Display, Write},
    sync::Arc,
};
use tracing_error::SpanTrace;
use user_facing_errors::{
    common::SchemaParserError, migration_engine::MigrationFileNotFound, KnownError, UserFacingError,
};

/// The general error reporting type for migration connectors.
#[derive(Clone)]
pub struct ConnectorError(Box<ConnectorErrorImpl>);

/// Shorthand for a [Result](https://doc.rust-lang.org/std/result/enum.Result.html) where the error
/// variant is a [ConnectorError](/error/enum.ConnectorError.html).
pub type ConnectorResult<T> = Result<T, ConnectorError>;

#[derive(Debug, Clone)]
struct ConnectorErrorImpl {
    /// An optional error already rendered for users in case the migration core does not handle it.
    user_facing_error: Option<KnownError>,
    /// Additional context.
    message: Option<Box<str>>,
    /// The source of the error.
    source: Option<Arc<(dyn StdError + Send + Sync + 'static)>>,
    /// See the tracing-error docs.
    context: SpanTrace,
}

impl Debug for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.0, f)?;
        f.write_char('\n')?;
        Display::fmt(self, f)
    }
}

impl Display for ConnectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.0.message {
            f.write_str(message)?;
            f.write_char('\n')?;
        }

        if let Some(source) = &self.0.source {
            Display::fmt(source.as_ref(), f)?;
            f.write_char('\n')?;
        }

        Display::fmt(&self.0.context, f)
    }
}

impl StdError for ConnectorError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.0
            .source
            .as_ref()
            .map(|err| -> &(dyn StdError + 'static) { err.as_ref() })
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

    /// Build a generic unknown error from just an error message.
    pub fn from_msg(msg: String) -> Self {
        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: None,
            context: SpanTrace::capture(),
            message: Some(msg.into_boxed_str()),
            source: None,
        }))
    }

    /// Build a generic unknown error from a source error, with some additional context.
    pub fn from_source<E: StdError + Send + Sync + 'static>(source: E, context: &'static str) -> Self {
        ConnectorError(Box::new(ConnectorErrorImpl {
            user_facing_error: None,
            message: Some(context.into()),
            source: Some(Arc::new(source)),
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
            source: Some(Arc::new(self)),
            context,
            message: None,
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
            message: None,
            context,
            source: Some(Arc::new(self)),
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
            context,
            message: None,
            source: Some(Arc::new(self)),
        }))
    }

    /// Access the inner `user_facing_error::KnownError`.
    pub fn known_error(&self) -> Option<&KnownError> {
        self.0.user_facing_error.as_ref()
    }

    /// Create a new P1012 user facing error from the rendered datamodel parser error.
    pub fn new_schema_parser_error(full_error: String) -> Self {
        ConnectorError::user_facing(SchemaParserError { full_error })
    }

    /// Render to a user_facing_error::Error
    pub fn to_user_facing(&self) -> user_facing_errors::Error {
        match &self.0.user_facing_error {
            Some(known_error) => known_error.clone().into(),
            None => user_facing_errors::Error::from_dyn_error(self),
        }
    }

    /// Construct a GenericError with an associated user facing error.
    pub fn user_facing<T: UserFacingError>(err: T) -> Self {
        ConnectorError(Box::new(ConnectorErrorImpl {
            message: Some(err.message().into_boxed_str()),
            user_facing_error: Some(KnownError::new(err)),
            source: None,
            context: SpanTrace::capture(),
        }))
    }

    /// Construct an UrlParseError.
    pub fn url_parse_error(err: impl Display) -> Self {
        Self::user_facing(user_facing_errors::common::InvalidConnectionString {
            details: err.to_string(),
        })
    }
}

impl From<KnownError> for ConnectorError {
    fn from(err: KnownError) -> Self {
        ConnectorError(Box::new(ConnectorErrorImpl {
            message: Some(err.message.clone().into_boxed_str()),
            user_facing_error: Some(err),
            source: None,
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
            context,
            message: None,
            source: Some(Arc::new(err)),
        }))
    }
}

impl From<ListMigrationsError> for ConnectorError {
    fn from(err: ListMigrationsError) -> Self {
        ConnectorError::from_msg(err.to_string())
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

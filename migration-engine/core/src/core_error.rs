use migration_connector::{ConnectorError, ListMigrationsError};
use std::{error::Error as StdError, fmt::Display};
use user_facing_errors::{common::SchemaParserError, KnownError, UserFacingError};

/// The result type for migration engine commands
pub type CoreResult<T> = Result<T, CoreError>;

/// The top-level error type for migration engine commands
#[derive(Debug)]
pub enum CoreError {
    /// Errors from the connector.
    ConnectorError(ConnectorError),

    /// User facing errors
    UserFacing(user_facing_errors::Error),
}

impl Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::ConnectorError(err) => write!(f, "Connector error: {:#}", err),
            CoreError::UserFacing(src) => f.write_str(src.message()),
        }
    }
}

impl StdError for CoreError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            CoreError::UserFacing(_) => None,
            CoreError::ConnectorError(err) => Some(err),
        }
    }
}

impl CoreError {
    /// Render to an `user_facing_error::Error`.
    pub fn render_user_facing(self) -> user_facing_errors::Error {
        match self {
            CoreError::ConnectorError(err) => err.to_user_facing(),
            CoreError::UserFacing(err) => err,
        }
    }

    pub(crate) fn new_schema_parser_error(full_error: String) -> Self {
        CoreError::user_facing(SchemaParserError { full_error })
    }

    pub(crate) fn new_unknown(message: String) -> Self {
        CoreError::UserFacing(user_facing_errors::Error::new_non_panic_with_current_backtrace(message))
    }

    /// Construct a user facing CoreError
    pub(crate) fn user_facing(error: impl UserFacingError) -> Self {
        CoreError::UserFacing(KnownError::new(error).into())
    }
}

impl From<ConnectorError> for CoreError {
    fn from(err: ConnectorError) -> Self {
        CoreError::ConnectorError(err)
    }
}

impl From<ListMigrationsError> for CoreError {
    fn from(err: ListMigrationsError) -> Self {
        CoreError::new_unknown(err.to_string())
    }
}

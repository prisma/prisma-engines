use migration_connector::{ConnectorError, ListMigrationsError};
use std::{error::Error as StdError, fmt::Display};
use user_facing_errors::{KnownError, UserFacingError};

/// The result type for migration engine commands
pub type CoreResult<T> = Result<T, CoreError>;

/// The top-level error type for migration engine commands
#[derive(Debug)]
pub enum CoreError {
    /// When there was a bad datamodel as part of the input
    ReceivedBadDatamodel(String),

    /// Errors from the connector.
    ConnectorError(ConnectorError),

    /// User facing errors
    UserFacing(user_facing_errors::Error),

    /// Generic unspecified errors
    Generic(anyhow::Error),
}

impl Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoreError::ReceivedBadDatamodel(err) => err.fmt(f),
            CoreError::ConnectorError(err) => write!(f, "Connector error: {:#}", err),
            CoreError::Generic(src) => src.fmt(f),
            CoreError::UserFacing(src) => f.write_str(src.message()),
        }
    }
}

impl StdError for CoreError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            CoreError::ReceivedBadDatamodel(_) => None,
            CoreError::UserFacing(_) => None,
            CoreError::ConnectorError(err) => Some(err),
            CoreError::Generic(err) => Some(err.as_ref()),
        }
    }
}

impl CoreError {
    /// Render to an `user_facing_error::Error`.
    pub fn render_user_facing(self) -> user_facing_errors::Error {
        match self {
            CoreError::ConnectorError(err) => err.to_user_facing(),
            CoreError::UserFacing(err) => err,
            CoreError::ReceivedBadDatamodel(full_error) => {
                KnownError::new(user_facing_errors::common::SchemaParserError { full_error }).into()
            }
            crate_error => user_facing_errors::Error::from_dyn_error(&crate_error),
        }
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
        CoreError::Generic(err.into())
    }
}

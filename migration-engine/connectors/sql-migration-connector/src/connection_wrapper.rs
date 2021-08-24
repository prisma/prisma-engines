mod native;

pub(crate) use native::*;

use migration_connector::ConnectorError;
use user_facing_errors::UserFacingError;

pub(crate) type SqlResult<T> = Result<T, SqlError>;

#[derive(Debug)]
pub(crate) struct SqlError {
    error_code: Option<String>,
    /// The constructed ConnectorError for bubbling up.
    connector_error: ConnectorError,
    /// A byte offset in the query text.
    src_position: Option<u32>,
    /// 0-based index of the statement in the original query.
    src_statement: Option<u32>,
}

impl SqlError {
    pub(crate) fn error_code(&self) -> Option<&str> {
        self.error_code.as_deref()
    }

    pub(crate) fn is_user_facing_error<T: UserFacingError>(&self) -> bool {
        self.connector_error.error_code() == Some(T::ERROR_CODE)
    }
}

impl From<SqlError> for ConnectorError {
    fn from(err: SqlError) -> Self {
        err.connector_error
    }
}

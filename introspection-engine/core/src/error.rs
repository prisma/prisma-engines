use failure::{Error, Fail};
use std::error::Error as StdError; // just bringing the trait functions into scope

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Fail)]
pub enum CoreError {
    #[fail(display = "Couldn't parse the connection string because of: {}", message)]
    InvalidUrl { message: String },
    #[fail(display = "Error in connector: {}", _0)]
    ConnectorError(Error),
}

impl From<url::ParseError> for CoreError {
    fn from(e: url::ParseError) -> Self {
        CoreError::InvalidUrl {
            message: format!("Couldn't parse the connection string because of: {}", e.description()),
        }
    }
}

impl From<introspection_connector::ConnectorError> for CoreError {
    fn from(e: introspection_connector::ConnectorError) -> Self {
        CoreError::ConnectorError(e.into())
    }
}

impl From<CoreError> for jsonrpc_core::types::error::Error {
    fn from(e: CoreError) -> Self {
        jsonrpc_core::types::error::Error {
            code: jsonrpc_core::ErrorCode::ServerError(1000),
            message: format!("CoreError: {}", e),
            data: None,
        }
    }
}

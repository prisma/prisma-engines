use failure::Fail;
use introspection_connector::ConnectorError;

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Fail)]
pub enum CoreError {
    #[fail(display = "Error in connector: {}", _0)]
    ConnectorError(#[fail(cause)] introspection_connector::ConnectorError),
    #[fail(display = "Datamodel rendering failed: {}", _0)]
    DatamodelRendering(datamodel::error::ErrorCollection),
}

impl From<ConnectorError> for CoreError {
    fn from(e: ConnectorError) -> Self {
        CoreError::ConnectorError(e)
    }
}

impl From<datamodel::error::ErrorCollection> for CoreError {
    fn from(e: datamodel::error::ErrorCollection) -> Self {
        CoreError::DatamodelRendering(e)
    }
}

impl From<CoreError> for jsonrpc_core::types::error::Error {
    fn from(mut e: CoreError) -> Self {
        use user_facing_errors::{KnownError, UnknownError};

        let known_error: Option<KnownError> = match &mut e {
            CoreError::ConnectorError(ConnectorError { user_facing, .. }) => user_facing.take(),
            _ => None,
        };

        let data: user_facing_errors::Error = known_error.map(user_facing_errors::Error::from).unwrap_or_else(|| {
            UnknownError {
                message: format!("{}", e),
                backtrace: e.backtrace().map(|bt| format!("{}", bt)),
            }
            .into()
        });

        jsonrpc_core::types::error::Error {
            code: jsonrpc_core::ErrorCode::ServerError(1000),
            message: format!("CoreError: {}", e),
            data: Some(serde_json::to_value(&data).unwrap()),
        }
    }
}

use failure::Fail;
use introspection_connector::ConnectorError;
use jsonrpc_core::types::error::Error as JsonRpcError;

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

        render_jsonrpc_error(data, format!("CoreError: {}", e))
    }
}

pub(crate) fn render_jsonrpc_error(payload: impl serde::Serialize, message: String) -> JsonRpcError {
    JsonRpcError {
        code: jsonrpc_core::ErrorCode::ServerError(1000),
        message,
        data: Some(serde_json::to_value(payload).unwrap()),
    }
}

pub(crate) fn render_panic(panic: Box<dyn std::any::Any + Send + 'static>) -> JsonRpcError {
    let error = user_facing_errors::UnknownError::from_panic_payload(panic.as_ref());
    render_jsonrpc_error(error, "Panicked during command handling.".into())
}

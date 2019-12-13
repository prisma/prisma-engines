use crate::{commands::CommandError, error::Error as CrateError};
use failure::Fail as _;
use jsonrpc_core::types::Error as JsonRpcError;
use migration_connector::ConnectorError;
use user_facing_errors::{Error, UnknownError};

pub fn render_error(crate_error: CrateError) -> Error {
    match crate_error {
        CrateError::ConnectorError(ConnectorError {
            user_facing_error: Some(user_facing_error),
            ..
        }) => user_facing_error.into(),
        CrateError::CommandError(CommandError::ConnectorError(ConnectorError {
            user_facing_error: Some(user_facing_error),
            ..
        })) => user_facing_error.into(),
        _ => UnknownError {
            message: format!("{}", crate_error),
            backtrace: crate_error.backtrace().map(|bt| format!("{}", bt)),
        }
        .into(),
    }
}

pub(super) fn render_jsonrpc_error(crate_error: CrateError) -> JsonRpcError {
    let prisma_error = render_error(crate_error);

    let error_rendering_result: Result<_, _> = match prisma_error {
        user_facing_errors::Error::Known(known) => serde_json::to_value(known).map(|data| {
            JsonRpcError {
                // We separate the JSON-RPC error code (defined by the JSON-RPC spec) from the
                // prisma error code, which is located in `data`.
                code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
                message: "An error happened. Check the data field for details.".to_string(),
                data: Some(data),
            }
        }),
        user_facing_errors::Error::Unknown(unknown) => Ok(render_unknown_error_as_jsonrpc_error(unknown)),
    };

    match error_rendering_result {
        Ok(err) => err,
        Err(err) => fallback_jsonrpc_error(err),
    }
}

pub(crate) fn render_panic(panic: Box<dyn std::any::Any + Send + 'static>) -> JsonRpcError {
    let error = user_facing_errors::UnknownError::from_panic_payload(panic.as_ref());
    render_unknown_error_as_jsonrpc_error(error)
}

fn render_unknown_error_as_jsonrpc_error(unknown_error: UnknownError) -> JsonRpcError {
    match serde_json::to_value(&unknown_error) {
        Ok(json_error) => JsonRpcError {
            code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
            message: "The migration engine panicked while handling the request. Check the data field for details."
                .to_string(),
            data: Some(json_error),
        },
        Err(err) => fallback_jsonrpc_error(err),
    }
}

/// Last-resort JSON-RPC error, when we can't even render the error.
fn fallback_jsonrpc_error(err: impl std::error::Error) -> JsonRpcError {
    tracing::warn!("Failed to render user facing error. Using fallback error.");

    JsonRpcError {
        code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
        message: "The migration engine encountered an error and failed to render it.".to_string(),
        data: Some(serde_json::json!({ "backtrace": null, "message": format!("{}", err) })),
    }
}

use crate::{commands::CommandError, error::Error as CoreError};
use jsonrpc_core::types::Error as JsonRpcError;
use migration_connector::ConnectorError;
use user_facing_errors::Error;

pub fn render_error(crate_error: CoreError) -> Error {
    match crate_error {
        CoreError::ConnectorError(ConnectorError {
            user_facing_error: Some(user_facing_error),
            ..
        }) => user_facing_error.into(),
        CoreError::CommandError(CommandError::ConnectorError(ConnectorError {
            user_facing_error: Some(user_facing_error),
            ..
        })) => user_facing_error.into(),
        _ => Error::from_fail(crate_error).into(),
    }
}

pub(super) fn render_jsonrpc_error(crate_error: CoreError) -> JsonRpcError {
    let prisma_error = render_error(crate_error);

    let error_rendering_result: Result<_, _> = serde_json::to_value(&prisma_error).map(|data| JsonRpcError {
        // We separate the JSON-RPC error code (defined by the JSON-RPC spec) from the
        // prisma error code, which is located in `data`.
        code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
        message: "An error happened. Check the data field for details.".to_string(),
        data: Some(data),
    });

    match error_rendering_result {
        Ok(err) => err,
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

pub(crate) fn pretty_print_datamodel_errors(
    errors: &datamodel::error::ErrorCollection,
    datamodel: &str,
) -> std::io::Result<String> {
    use std::io::Write as _;

    let file_name = "schema.prisma";

    let mut message: Vec<u8> = Vec::new();

    for error in errors.to_iter() {
        writeln!(&mut message)?;
        error
            .pretty_print(&mut message, file_name, datamodel)
            .expect("Failed to write errors to stderr");
    }

    Ok(String::from_utf8_lossy(&message).into_owned())
}

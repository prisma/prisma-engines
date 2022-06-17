use crate::error::Error;
use introspection_connector::ConnectorError;
use jsonrpc_core::types::Error as JsonRpcError;
use user_facing_errors::{
    common::SchemaParserError, introspection_engine::IntrospectionResultEmpty, Error as UserFacingError, KnownError,
};

pub fn render_error(crate_error: Error) -> UserFacingError {
    match crate_error {
        Error::ConnectorError(ConnectorError {
            user_facing_error: Some(user_facing_error),
            ..
        }) => user_facing_error.into(),
        Error::IntrospectionResultEmpty => KnownError::new(IntrospectionResultEmpty).into(),
        Error::DatamodelError(full_error) => KnownError::new(SchemaParserError { full_error }).into(),
        _ => UserFacingError::from_dyn_error(&crate_error),
    }
}

impl From<Error> for JsonRpcError {
    fn from(e: Error) -> Self {
        let prisma_error = render_error(e);

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

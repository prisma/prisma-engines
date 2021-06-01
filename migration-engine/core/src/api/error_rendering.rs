use crate::CoreError;
use jsonrpc_core::types::Error as JsonRpcError;

pub(super) fn render_jsonrpc_error(crate_error: CoreError) -> JsonRpcError {
    serde_json::to_value(&crate_error.to_user_facing())
        .map(|data| JsonRpcError {
            // We separate the JSON-RPC error code (defined by the JSON-RPC spec) from the
            // prisma error code, which is located in `data`.
            code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
            message: "An error happened. Check the data field for details.".to_string(),
            data: Some(data),
        })
        .unwrap()
}

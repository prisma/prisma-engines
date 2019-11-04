use crate::error::Error as CrateError;
use failure::Fail as _;
use jsonrpc_core::types::Error as JsonRpcError;
use migration_connector::ConnectorError;
use user_facing_errors::{Error, KnownError, UnknownError};

pub fn render_error(crate_error: CrateError) -> Error {
    let result: Result<Error, _> = match &crate_error {
        CrateError::ConnectorError(ConnectorError::AuthenticationFailed { user, host }) => {
            KnownError::new(user_facing_errors::common::IncorrectDatabaseCredentials {
                database_user: user.clone(),
                database_host: host.clone(),
            })
            .map(Error::Known)
        }
        CrateError::ConnectorError(ConnectorError::ConnectionError { host, port, .. }) => {
            KnownError::new(user_facing_errors::common::DatabaseNotReachable {
                database_host: host.clone(),
                database_port: port
                    .map(|port| format!("{}", port))
                    .unwrap_or_else(|| format!("<unknown>")),
            })
            .map(Error::Known)
        }
        CrateError::ConnectorError(ConnectorError::DatabaseDoesNotExist {
            db_name,
            database_location,
        }) => KnownError::new(user_facing_errors::common::DatabaseDoesNotExist {
            database_name: db_name.clone(),
            database_location: database_location.clone(),
            database_schema_name: None,
        })
        .map(Error::Known),
        CrateError::ConnectorError(ConnectorError::DatabaseAccessDenied {
            database_user,
            database_name,
        }) => KnownError::new(user_facing_errors::common::DatabaseAccessDenied {
            database_user: database_user.clone(),
            database_name: database_name.clone(),
        })
        .map(Error::Known),
        err => Ok(UnknownError {
            message: format!("{}", err),
            backtrace: None,
        }
        .into()),
    };

    match result {
        Ok(error) => error,
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

pub(crate) fn render_panic(panic: Box<dyn std::any::Any>) -> JsonRpcError {
    let error_message: Option<String> = panic
        .downcast_ref::<&'static str>()
        .map(|s| (*s).to_owned())
        .or_else(|| panic.downcast_ref::<String>().map(|s| s.to_owned()));

    let error = user_facing_errors::UnknownError {
        message: error_message.unwrap_or_else(|| "Error rendering Rust panic.".to_owned()),
        backtrace: None,
    };

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
    log::warn!("Failed to render user facing error. Using fallback error.");

    JsonRpcError {
        code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
        message: "The migration engine encountered an error and failed to render it.".to_string(),
        data: Some(serde_json::json!({ "backtrace": null, "message": format!("{}", err) })),
    }
}

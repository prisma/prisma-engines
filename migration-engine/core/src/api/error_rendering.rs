use crate::commands::CommandError;
use crate::error::Error as CrateError;
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
    log::warn!("Failed to render user facing error. Using fallback error.");

    JsonRpcError {
        code: jsonrpc_core::types::error::ErrorCode::ServerError(4466),
        message: "The migration engine encountered an error and failed to render it.".to_string(),
        data: Some(serde_json::json!({ "backtrace": null, "message": format!("{}", err) })),
    }
}

fn render_connector_error(connector_error: &ConnectorError) -> Result<Error, serde_json::Error> {
    match connector_error {
        ConnectorError::AuthenticationFailed { user, host } => {
            KnownError::new(user_facing_errors::common::IncorrectDatabaseCredentials {
                database_user: user.clone(),
                database_host: host.clone(),
            })
            .map(Error::Known)
        }

        ConnectorError::ConnectionError { host, port, .. } => {
            KnownError::new(user_facing_errors::common::DatabaseNotReachable {
                database_host: host.clone(),
                database_port: port
                    .map(|port| format!("{}", port))
                    .unwrap_or_else(|| format!("<unknown>")),
            })
            .map(Error::Known)
        }

        ConnectorError::DatabaseDoesNotExist {
            db_name,
            database_location,
        } => KnownError::new(user_facing_errors::common::DatabaseDoesNotExist {
            database_name: db_name.clone(),
            database_location: database_location.clone(),
            database_schema_name: None,
        })
        .map(Error::Known),

        ConnectorError::DatabaseAccessDenied {
            database_user,
            database_name,
        } => KnownError::new(user_facing_errors::common::DatabaseAccessDenied {
            database_user: database_user.clone(),
            database_name: database_name.clone(),
        })
        .map(Error::Known),

        ConnectorError::UniqueConstraintViolation { field_name } => {
            KnownError::new(user_facing_errors::query_engine::UniqueKeyViolation {
                field_name: field_name.clone(),
            })
            .map(Error::Known)
        }
        _ => Ok(UnknownError {
            message: format!("{}", connector_error),
            backtrace: connector_error.backtrace().map(|bt| format!("{}", bt)),
        }
        .into()),
    }
}

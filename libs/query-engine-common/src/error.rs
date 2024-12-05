use psl::diagnostics::Diagnostics;
use query_connector::error::ConnectorError;
use query_core::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{:?}", _0)]
    Conversion(Diagnostics, String),

    #[error("{}", _0)]
    Configuration(String),

    #[error("{}", _0)]
    Core(CoreError),

    #[error("{}", _0)]
    Connector(ConnectorError),

    #[error("Can't modify an already connected engine.")]
    AlreadyConnected,

    #[error("Engine is not yet connected.")]
    NotConnected,

    #[error("{}", _0)]
    Decode(String),
}

impl From<ApiError> for user_facing_errors::Error {
    fn from(err: ApiError) -> Self {
        use std::fmt::Write as _;

        match err {
            ApiError::Connector(ConnectorError {
                user_facing_error: Some(err),
                ..
            }) => err.into(),
            ApiError::Conversion(errors, dml_string) => {
                let mut full_error = errors.to_pretty_string("schema.prisma", &dml_string);
                write!(full_error, "\nValidation Error Count: {}", errors.errors().len()).unwrap();

                user_facing_errors::Error::from(user_facing_errors::KnownError::new(
                    user_facing_errors::common::SchemaParserError { full_error },
                ))
            }
            ApiError::Core(error) => user_facing_errors::Error::from(error),
            other => user_facing_errors::Error::new_non_panic_with_current_backtrace(other.to_string()),
        }
    }
}

impl ApiError {
    pub fn conversion(diagnostics: Diagnostics, dml: impl ToString) -> Self {
        Self::Conversion(diagnostics, dml.to_string())
    }

    pub fn configuration(msg: impl ToString) -> Self {
        Self::Configuration(msg.to_string())
    }
}

impl From<CoreError> for ApiError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::ConfigurationError(message) => Self::Configuration(message),
            core_error => Self::Core(core_error),
        }
    }
}

impl From<ConnectorError> for ApiError {
    fn from(e: ConnectorError) -> Self {
        Self::Connector(e)
    }
}

impl From<url::ParseError> for ApiError {
    fn from(e: url::ParseError) -> Self {
        Self::configuration(format!("Error parsing connection string: {e}"))
    }
}

impl From<connection_string::Error> for ApiError {
    fn from(e: connection_string::Error) -> Self {
        Self::configuration(format!("Error parsing connection string: {e}"))
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        Self::Decode(format!("{e}"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(any(target_os = "android", target_os = "ios")))]
impl From<ApiError> for napi::Error {
    fn from(e: ApiError) -> Self {
        let user_facing = user_facing_errors::Error::from(e);
        let message = serde_json::to_string(&user_facing).unwrap();

        napi::Error::from_reason(message)
    }
}

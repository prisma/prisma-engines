use crate::TemplatingError;
use quaint::error::Error as QuaintError;
use std::env::VarError;
use thiserror::Error;
use user_facing_errors::query_engine::validation::ValidationError;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum TestError {
    #[error("Handler Error: {0}")]
    RequestHandlerError(request_handlers::HandlerError),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Query core error: {0}")]
    QueryCoreError(#[from] query_core::CoreError),

    #[error("Migration core error: {0}")]
    MigrationCoreError(#[from] qe_setup::ConnectorError),

    #[error("Error processing schema template: {0}")]
    TemplatingError(#[from] TemplatingError),

    #[error("Error during interactive transaction processing: {}", _0)]
    InteractiveTransactionError(String),

    #[error("Raw execute error: {0}")]
    RawExecute(QuaintError),

    #[error("External process error: {0}")]
    External(#[from] Box<dyn std::error::Error + Send + Sync>),

    #[error("Error converting GraphQL query to JSON: {0}")]
    QueryConversionError(#[from] ValidationError),
}

impl TestError {
    pub fn parse_error<T>(reason: T) -> Self
    where
        T: Into<String>,
    {
        Self::ParseError(reason.into())
    }

    pub fn config_error<T>(reason: T) -> Self
    where
        T: Into<String>,
    {
        Self::ConfigError(reason.into())
    }
}

impl From<VarError> for TestError {
    fn from(err: VarError) -> Self {
        Self::ConfigError(err.to_string())
    }
}

impl From<QuaintError> for TestError {
    fn from(err: QuaintError) -> Self {
        Self::RawExecute(err)
    }
}

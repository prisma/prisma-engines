use crate::TemplatingError;
use std::env::VarError;
use thiserror::Error;

#[allow(clippy::large_enum_variant)]
#[derive(Debug, Error)]
pub enum TestError {
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

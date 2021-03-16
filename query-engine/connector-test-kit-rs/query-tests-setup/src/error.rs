use std::env::VarError;

#[derive(Debug)]
pub enum TestError {
    ParseError(String),
    ConfigError(String),
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

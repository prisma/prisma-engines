use graphql_parser::query::ParseError;
use query_core::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HandlerError {
    #[error("{}", _0)]
    Core(#[from] CoreError),

    #[error("{}", _0)]
    Configuration(String),

    #[error("{}", _0)]
    QueryConversion(String),

    #[error("Unsupported feature: {}. {}", feature_name, message)]
    UnsupportedFeature {
        feature_name: &'static str,
        message: String,
    },
}

impl HandlerError {
    pub fn configuration(message: impl ToString) -> Self {
        Self::Configuration(message.to_string())
    }

    pub fn query_conversion(message: impl ToString) -> Self {
        Self::Configuration(message.to_string())
    }

    pub fn unsupported_feature(feature_name: &'static str, message: impl ToString) -> Self {
        let message = message.to_string();

        Self::UnsupportedFeature { feature_name, message }
    }
}

impl From<url::ParseError> for HandlerError {
    fn from(e: url::ParseError) -> Self {
        Self::configuration(format!("Error parsing connection string: {}", e))
    }
}

impl From<connection_string::Error> for HandlerError {
    fn from(e: connection_string::Error) -> Self {
        Self::configuration(format!("Error parsing connection string: {}", e))
    }
}

impl From<ParseError> for HandlerError {
    fn from(e: ParseError) -> Self {
        Self::configuration(format!("Error parsing GraphQL query: {}", e))
    }
}

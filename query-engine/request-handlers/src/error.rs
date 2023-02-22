use query_core::CoreError;
use serde::Serialize;
use thiserror::Error;
use user_facing_errors::KnownError;

#[derive(Debug, Error)]
#[allow(clippy::large_enum_variant)]
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

    #[error("{}", _0)]
    ValueFitError(String),
}

// Implements a custom serializer to provide a structured json format for validation errors.
// Custom serializers are implemented down CoreError through QueryGraphBuilderError to ParseError
// where the errors messages have a well-defined json structure.
//
// Different error variants can progressively implement a typed structured that can be serialized
// as JSON. Until that migration is complete, their json representation will be the error message
// as stringified JSON. (Given the error message was what was returned to the client before any kind
// of JSON structure for messages was considered)
impl Serialize for HandlerError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            HandlerError::Core(e) => e.serialize(serializer),
            err @ _ => serializer.serialize_str(format!("{}", err).as_str()),
        }
    }
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

    pub fn value_fit(details: impl ToString) -> Self {
        Self::ValueFitError(details.to_string())
    }

    pub fn as_known_error(&self) -> Option<KnownError> {
        match self {
            HandlerError::ValueFitError(details) => {
                Some(KnownError::new(user_facing_errors::query_engine::ValueFitError {
                    details: details.clone(),
                }))
            }
            _ => None,
        }
    }
}

impl From<url::ParseError> for HandlerError {
    fn from(e: url::ParseError) -> Self {
        Self::configuration(format!("Error parsing connection string: {e}"))
    }
}

impl From<connection_string::Error> for HandlerError {
    fn from(e: connection_string::Error) -> Self {
        Self::configuration(format!("Error parsing connection string: {e}"))
    }
}

impl From<graphql_parser::query::ParseError> for HandlerError {
    fn from(e: graphql_parser::query::ParseError) -> Self {
        Self::configuration(format!("Error parsing GraphQL query: {e}"))
    }
}

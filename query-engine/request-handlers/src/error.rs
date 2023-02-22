use query_core::{query_graph_builder::QueryGraphBuilderError, CoreError, QueryParserError};
use thiserror::Error;
use user_facing_errors::{KnownError, UnknownError};

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

// FIXME: Prior to the introduction of structured validation errors, any HandlerError was an
// [user_facing_error::UnknownError].
//
// In addition, there's a specific variant of HandlerErrors, [HandlerError::Core] that wraps a
// CoreError. An existing implementation of From<CoreError> for a [user_facing_error::Error]
// converts some of the CoreError variants to specific user facing errors, but
// this method you are reading, was not using that implementation at all, and instead created
// [user_facing_errors::UnknownError] for every HandleError, including the Core variants
// that would could be converted to [user_facing_errors::KnownError] values.
//
// When adding the validation, we want those to be values of [user_facing::KnownError] type i.e.
// to have a proper error code and arbitrary metadata. We could leverage the From<CoreError> for
// [user_facing_error::Error], but that means that in addition to the validation errors, the client
// would see some other core errors to change the format, thus leading to a breaking change.
//
// The path forward would be to gradually:
//
// - Implement the UserFacingError trait for CoreError variants.
// - Refactor From<CoreError> to [user_facing_error::Error] to replace each variant case to be a no-op
// - Add a case clause in this trait's `from` function for each error that now has been migrating
//   and adapt the ts client to consume it, because now it will have a different format.
// - Once all variants have been refactored, change this method to instead reuse the From<CoreError>
//   trait implementation on [user_facing_error::Error].
// - Repeat the above strategy for other variantes of a [HandlerError] and if possible, flatten
//   the nested structure of variants.
impl From<HandlerError> for user_facing_errors::Error {
    fn from(err: HandlerError) -> Self {
        match err {
            // core errors, for them mmoment we only convert parsing errors as known errors, to
            // not brake the client. Ideally we would delegate to [user_facing_errors::Error::from]
            // for core all errors.
            HandlerError::Core(ref ce) => match ce {
                CoreError::QueryParserError(QueryParserError::Structured(se))
                | CoreError::QueryGraphBuilderError(QueryGraphBuilderError::QueryParserError(
                    QueryParserError::Structured(se),
                )) => KnownError::new(se.to_owned()).into(),
                _ => UnknownError::new(&err).into(),
            },

            // value fit error
            HandlerError::ValueFitError(details) => KnownError::new(user_facing_errors::query_engine::ValueFitError {
                details: details.clone(),
            })
            .into(),

            _ => UnknownError::new(&err).into(),
        }
    }
}

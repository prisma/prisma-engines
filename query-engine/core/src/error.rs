use crate::{InterpreterError, QueryGraphBuilderError, QueryGraphError, QueryParserError};
use connector::error::ConnectorError;
use failure::Fail;
use prisma_models::DomainError;

// TODO: Cleanup unused errors after refactorings.
#[derive(Debug, Fail)]
pub enum CoreError {
    #[fail(display = "Error in query graph construction: {:?}", _0)]
    QueryGraphError(QueryGraphError),

    #[fail(display = "Error in query graph construction: {:?}", _0)]
    QueryGraphBuilderError(QueryGraphBuilderError),

    #[fail(display = "Error in connector: {}", _0)]
    ConnectorError(ConnectorError),

    #[fail(display = "Error in domain logic: {}", _0)]
    DomainError(DomainError),

    #[fail(display = "{}", _0)]
    QueryParserError(QueryParserError),

    #[fail(display = "Unsupported feature: {}", _0)]
    UnsupportedFeatureError(String),

    #[fail(display = "{}", _0)]
    ConversionError(String),

    #[fail(display = "{}", _0)]
    SerializationError(String),

    #[fail(display = "{}", _0)]
    InterpreterError(InterpreterError),
}

impl From<QueryGraphBuilderError> for CoreError {
    fn from(e: QueryGraphBuilderError) -> CoreError {
        CoreError::QueryGraphBuilderError(e)
    }
}

impl From<QueryGraphError> for CoreError {
    fn from(e: QueryGraphError) -> CoreError {
        CoreError::QueryGraphError(e)
    }
}

impl From<ConnectorError> for CoreError {
    fn from(e: ConnectorError) -> CoreError {
        CoreError::ConnectorError(e)
    }
}

impl From<DomainError> for CoreError {
    fn from(e: DomainError) -> CoreError {
        CoreError::DomainError(e)
    }
}

impl From<QueryParserError> for CoreError {
    fn from(e: QueryParserError) -> CoreError {
        CoreError::QueryParserError(e)
    }
}

impl From<InterpreterError> for CoreError {
    fn from(e: InterpreterError) -> CoreError {
        CoreError::InterpreterError(e)
    }
}

impl From<CoreError> for user_facing_errors::Error {
    fn from(err: CoreError) -> user_facing_errors::Error {
        match err {
            CoreError::ConnectorError(ConnectorError {
                user_facing_error: Some(user_facing_error),
                ..
            })
            | CoreError::InterpreterError(InterpreterError::ConnectorError(ConnectorError {
                user_facing_error: Some(user_facing_error),
                ..
            })) => user_facing_error.into(),
            CoreError::QueryParserError(query_parser_error)
            | CoreError::QueryGraphBuilderError(QueryGraphBuilderError::QueryParserError(query_parser_error)) => {
                user_facing_errors::KnownError::new(user_facing_errors::query_engine::QueryValidationFailed {
                    query_validation_error: format!("{}", query_parser_error),
                    query_position: format!("{}", query_parser_error.location()),
                })
                .unwrap()
                .into()
            }
            _ => user_facing_errors::UnknownError::from_fail(err).into(),
        }
    }
}

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

// temporary
impl Into<ConnectorError> for CoreError {
    fn into(self) -> ConnectorError {
        ConnectorError::CoreError(format!("{}", self))
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

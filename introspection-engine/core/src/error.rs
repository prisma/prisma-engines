use failure::{Error, Fail};
use std::error::Error as StdError; // just bringing the trait functions into scope

pub type CoreResult<T> = Result<T, CoreError>;

#[derive(Debug, Fail)]
pub enum CoreError {
    #[fail(display = "Couldn't parse the connection string because of: {}", message)]
    InvalidUrl { message: String },
    #[fail(display = "Error in connector: {}", _0)]
    ConnetorError(Error),
}

impl From<url::ParseError> for CoreError {
    fn from(e: url::ParseError) -> Self {
        CoreError::InvalidUrl {
            message: format!("Couldn't parse the connection string because of: {}", e.description()),
        }
    }
}

impl From<prisma_query::error::Error> for CoreError {
    fn from(e: prisma_query::error::Error) -> Self {
        CoreError::ConnetorError(e.into())
    }
}

impl From<sql_schema_describer::SqlSchemaDescriberError> for CoreError {
    fn from(e: sql_schema_describer::SqlSchemaDescriberError) -> Self {
        CoreError::ConnetorError(e.into())
    }
}

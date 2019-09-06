use failure::{Error, Fail};
use introspection_connector::ConnectorError;
use std::error::Error as StdError; // just bringing the trait functions into scope

#[derive(Debug, Fail)]
pub enum SqlIntrospectionError {
    #[fail(display = "Couldn't parse the connection string because of: {}", message)]
    InvalidUrl { message: String },
    #[fail(display = "{}", _0)]
    Generic(Error),
}

impl From<url::ParseError> for SqlIntrospectionError {
    fn from(e: url::ParseError) -> Self {
        SqlIntrospectionError::InvalidUrl {
            message: format!("Couldn't parse the connection string because of: {}", e.description()),
        }
    }
}

impl From<prisma_query::error::Error> for SqlIntrospectionError {
    fn from(e: prisma_query::error::Error) -> Self {
        SqlIntrospectionError::Generic(e.into())
    }
}

impl From<sql_schema_describer::SqlSchemaDescriberError> for SqlIntrospectionError {
    fn from(e: sql_schema_describer::SqlSchemaDescriberError) -> Self {
        SqlIntrospectionError::Generic(e.into())
    }
}

impl From<SqlIntrospectionError> for ConnectorError {
    fn from(error: SqlIntrospectionError) -> Self {
        ConnectorError::Generic(error.into())
    }
}

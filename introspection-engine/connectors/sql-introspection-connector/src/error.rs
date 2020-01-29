use failure::{Error, Fail};
use introspection_connector::{ConnectorError, ErrorKind};
use quaint::{error::Error as QuaintError, prelude::ConnectionInfo};
use user_facing_errors::KnownError;

#[derive(Debug, Fail)]
pub enum SqlIntrospectionError {
    #[fail(display = "Couldn't parse the connection string because of: {}", message)]
    InvalidUrl { message: String },
    #[fail(display = "{}", _0)]
    Generic(Error),
    #[fail(display = "{}", _0)]
    Quaint(QuaintError),
}

impl From<url::ParseError> for SqlIntrospectionError {
    fn from(e: url::ParseError) -> Self {
        SqlIntrospectionError::InvalidUrl {
            message: format!("Couldn't parse the connection string because of: {}", e),
        }
    }
}

impl From<quaint::error::Error> for SqlIntrospectionError {
    fn from(e: quaint::error::Error) -> Self {
        SqlIntrospectionError::Generic(e.into())
    }
}

impl From<sql_schema_describer::SqlSchemaDescriberError> for SqlIntrospectionError {
    fn from(e: sql_schema_describer::SqlSchemaDescriberError) -> Self {
        SqlIntrospectionError::Generic(e.into())
    }
}

impl SqlIntrospectionError {
    pub(crate) fn into_connector_error(self, connection_info: &ConnectionInfo) -> ConnectorError {
        let user_facing = match &self {
            SqlIntrospectionError::Quaint(quaint_error) => {
                user_facing_errors::quaint::render_quaint_error(quaint_error, connection_info)
            }
            err => KnownError::new(user_facing_errors::introspection_engine::IntrospectionFailed {
                introspection_error: format!("{}", err),
            })
            .ok(),
        };

        ConnectorError {
            user_facing,
            kind: ErrorKind::Generic(self.into()),
        }
    }
}

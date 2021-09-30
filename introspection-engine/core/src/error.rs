use introspection_connector::{ConnectorError, ErrorKind};
use std::fmt::Display;

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
pub enum Error {
    ConnectorError(ConnectorError),
    /// When there was a bad datamodel as part of the input.
    DatamodelError(String),
    /// A generic error.
    Generic(String),
    InvalidDatabaseUrl(String),
    /// When there are no models or enums detected.
    IntrospectionResultEmpty(String),
    /// Preview feature was not enabled
    PreviewFeatureNotEnabled(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::ConnectorError(err) => write!(f, "Error in connector: {}", err),
            Error::DatamodelError(err) => write!(f, "Error in datamodel:\n{}", err),
            Error::InvalidDatabaseUrl(err) => f.write_str(err),
            Error::IntrospectionResultEmpty(details) => {
                f.write_str("The introspected database was empty: ")?;
                f.write_str(details)
            }
            Error::Generic(err) => f.write_str(err),
            Error::PreviewFeatureNotEnabled(err) => f.write_str(err),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::ConnectorError(err) => Some(err),
            Error::DatamodelError(_) => None,
            Error::InvalidDatabaseUrl(_) => None,
            Error::IntrospectionResultEmpty(_) => None,
            Error::Generic(_) => None,
            Error::PreviewFeatureNotEnabled(_) => None,
        }
    }
}

impl From<ConnectorError> for Error {
    fn from(e: ConnectorError) -> Self {
        match e.kind {
            ErrorKind::InvalidDatabaseUrl(reason) => Self::InvalidDatabaseUrl(reason),
            e @ ErrorKind::PreviewFeatureNotEnabled(_) => Self::PreviewFeatureNotEnabled(e.to_string()),
            _ => Error::ConnectorError(e),
        }
    }
}

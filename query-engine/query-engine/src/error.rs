use connector::error::ConnectorError;
use psl::diagnostics::Diagnostics;
use query_core::CoreError;
use thiserror::Error;

#[derive(Debug, Error)]
#[allow(clippy::enum_variant_names)]
pub enum PrismaError {
    #[error(transparent)]
    CoreError(Box<CoreError>),

    #[error(transparent)]
    JsonDecodeError(anyhow::Error),

    #[error("{}", _0)]
    ConfigurationError(String),

    #[error(transparent)]
    ConnectorError(Box<ConnectorError>),

    #[error("{:?}", _0)]
    ConversionError(Diagnostics, String),

    #[error(transparent)]
    IOError(anyhow::Error),

    #[error("Error in data model: {:?}", _0)]
    DatamodelError(Diagnostics),
}

impl From<PrismaError> for user_facing_errors::Error {
    fn from(err: PrismaError) -> Self {
        use std::fmt::Write as _;

        match err {
            PrismaError::ConnectorError(connector_err) => match *connector_err {
                ConnectorError {
                    user_facing_error: Some(err),
                    ..
                } => (*err).into(),
                other => {
                    let err = PrismaError::ConnectorError(Box::new(other));
                    user_facing_errors::Error::new_non_panic_with_current_backtrace(err.to_string())
                }
            },
            PrismaError::ConversionError(errors, dml_string) => {
                let mut full_error = errors.to_pretty_string("schema.prisma", &dml_string);
                write!(full_error, "\nValidation Error Count: {}", errors.errors().len()).unwrap();

                user_facing_errors::Error::from(user_facing_errors::KnownError::new(
                    user_facing_errors::common::SchemaParserError { full_error },
                ))
            }
            other => user_facing_errors::Error::new_non_panic_with_current_backtrace(other.to_string()),
        }
    }
}

impl PrismaError {
    pub fn render_as_json(self) -> Result<(), anyhow::Error> {
        use std::io::Write as _;

        let error = user_facing_errors::Error::from(self);

        // Because of how the node frontend works (stderr.on('data', ...)), we want to emit one clean JSON message on a single line at once.
        let stderr = std::io::stderr();
        let locked_stderr = stderr.lock();
        let mut writer = std::io::LineWriter::new(locked_stderr);
        serde_json::to_writer(&mut writer, &error)?;
        writeln!(&mut writer)?;
        writer.flush()?;

        Ok(())
    }
}

impl From<CoreError> for PrismaError {
    fn from(e: CoreError) -> Self {
        match e {
            CoreError::ConnectorError(e) => Self::ConnectorError(Box::new(e)),
            CoreError::ConfigurationError(message) => Self::ConfigurationError(message),
            _ => PrismaError::CoreError(Box::new(e)),
        }
    }
}

impl From<Diagnostics> for PrismaError {
    fn from(e: Diagnostics) -> Self {
        PrismaError::DatamodelError(e)
    }
}

impl From<url::ParseError> for PrismaError {
    fn from(e: url::ParseError) -> PrismaError {
        PrismaError::ConfigurationError(format!("Error parsing connection string: {e}"))
    }
}

impl From<connection_string::Error> for PrismaError {
    fn from(e: connection_string::Error) -> PrismaError {
        PrismaError::ConfigurationError(format!("Error parsing connection string: {e}"))
    }
}

impl From<serde_json::error::Error> for PrismaError {
    fn from(e: serde_json::error::Error) -> PrismaError {
        PrismaError::JsonDecodeError(e.into())
    }
}

impl From<std::io::Error> for PrismaError {
    fn from(e: std::io::Error) -> PrismaError {
        PrismaError::IOError(e.into())
    }
}

impl From<std::string::FromUtf8Error> for PrismaError {
    fn from(e: std::string::FromUtf8Error) -> PrismaError {
        PrismaError::IOError(e.into())
    }
}

impl From<base64::DecodeError> for PrismaError {
    fn from(e: base64::DecodeError) -> PrismaError {
        PrismaError::ConfigurationError(format!("Invalid base64: {e}"))
    }
}

impl From<ConnectorError> for PrismaError {
    fn from(e: ConnectorError) -> PrismaError {
        PrismaError::ConnectorError(Box::new(e))
    }
}

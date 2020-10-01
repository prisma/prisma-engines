use connector::error::ConnectorError;
use datamodel::error::ErrorCollection;
use feature_flags::FeatureFlagError;
use graphql_parser::query::ParseError as GqlParseError;
use query_core::CoreError;
use serde_json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PrismaError {
    #[error("{}", _0)]
    SerializationError(String),

    #[error("{}", _0)]
    CoreError(CoreError),

    #[error("{}", _0)]
    JsonDecodeError(anyhow::Error),

    #[error("{}", _0)]
    ConfigurationError(String),

    #[error("{}", _0)]
    ConnectorError(ConnectorError),

    #[error("{}", _0)]
    ConversionError(ErrorCollection, String),

    #[error("{}", _0)]
    IOError(anyhow::Error),

    #[error("{}", _0)]
    InvocationError(String),

    /// (Feature name, additional error text)
    #[error("Unsupported feature: {}. {}", _0, _1)]
    UnsupportedFeatureError(&'static str, String),

    #[error("Error in data model: {}", _0)]
    DatamodelError(ErrorCollection),

    #[error("{}", _0)]
    QueryConversionError(String),

    #[error("{}", _0)]
    FeatureError(String),
}

impl PrismaError {
    pub(crate) fn render_as_json(self) -> Result<(), anyhow::Error> {
        use std::fmt::Write as _;
        use std::io::Write as _;

        let error: user_facing_errors::Error = match self {
            PrismaError::ConnectorError(ConnectorError {
                user_facing_error: Some(err),
                ..
            }) => err.into(),
            PrismaError::ConversionError(errors, dml_string) => {
                let mut full_error = errors.to_pretty_string("schema.prisma", &dml_string);
                write!(full_error, "\nValidation Error Count: {}", errors.to_iter().len())?;

                user_facing_errors::Error::from(user_facing_errors::KnownError::new(
                    user_facing_errors::common::SchemaParserError { full_error },
                ))
            }
            other => user_facing_errors::Error::new_non_panic_with_current_backtrace(other.to_string()),
        };

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
        PrismaError::CoreError(e)
    }
}

impl From<ErrorCollection> for PrismaError {
    fn from(e: ErrorCollection) -> Self {
        PrismaError::DatamodelError(e)
    }
}

impl From<url::ParseError> for PrismaError {
    fn from(e: url::ParseError) -> PrismaError {
        PrismaError::ConfigurationError(format!("Error parsing connection string: {}", e))
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
        PrismaError::ConfigurationError(format!("Invalid base64: {}", e))
    }
}

impl From<GqlParseError> for PrismaError {
    fn from(e: GqlParseError) -> PrismaError {
        PrismaError::QueryConversionError(format!("Error parsing GraphQL query: {}", e))
    }
}

impl From<ConnectorError> for PrismaError {
    fn from(e: ConnectorError) -> PrismaError {
        PrismaError::ConnectorError(e)
    }
}

impl From<FeatureFlagError> for PrismaError {
    fn from(e: FeatureFlagError) -> Self {
        PrismaError::FeatureError(e.to_string())
    }
}

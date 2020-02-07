use datamodel::error::ErrorCollection;
use failure::{Error, Fail};
use graphql_parser::query::ParseError as GqlParseError;
use query_core::{response_ir, CoreError};
use serde_json;

#[cfg(feature = "sql")]
use sql_connector::SqlError;

#[derive(Debug, Fail)]
pub enum PrismaError {
    #[fail(display = "{}", _0)]
    SerializationError(String),

    #[fail(display = "{}", _0)]
    CoreError(CoreError),

    #[fail(display = "{}", _0)]
    JsonDecodeError(Error),

    #[fail(display = "{}", _0)]
    ConfigurationError(String),

    #[fail(display = "{}", _0)]
    ConversionError(ErrorCollection, String),

    #[fail(display = "{}", _0)]
    IOError(Error),

    #[fail(display = "{}", _0)]
    InvocationError(String),

    /// (Feature name, additional error text)
    #[fail(display = "Unsupported feature: {}. {}", _0, _1)]
    UnsupportedFeatureError(&'static str, String),

    #[fail(display = "Error in data model: {}", _0)]
    DatamodelError(ErrorCollection),

    #[fail(display = "{}", _0)]
    QueryConversionError(String),
}

impl PrismaError {
    pub(crate) fn render_as_json(&self) -> Result<(), failure::Error> {
        use std::fmt::Write as _;
        use std::io::Write as _;

        let error: user_facing_errors::Error = match self {
            PrismaError::ConversionError(errors, dml_string) => {
                let mut full_error = errors.to_pretty_string("schema.prisma", dml_string);
                write!(full_error, "\nValidation Error Count: {}", errors.to_iter().len())?;

                user_facing_errors::Error::from(
                    user_facing_errors::KnownError::new(user_facing_errors::common::SchemaParserError { full_error })
                        .unwrap(),
                )
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

/// Helps to handle gracefully handle errors as a response.
impl From<PrismaError> for response_ir::ResponseError {
    fn from(other: PrismaError) -> Self {
        match other {
            PrismaError::CoreError(core_error) => response_ir::ResponseError::from(core_error),
            err => response_ir::ResponseError::from(user_facing_errors::Error::from_fail(err)),
        }
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

#[cfg(feature = "sql")]
impl From<SqlError> for PrismaError {
    fn from(e: SqlError) -> PrismaError {
        PrismaError::ConfigurationError(format!("{}", e))
    }
}

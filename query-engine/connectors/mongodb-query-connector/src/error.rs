use connector_interface::error::{ConnectorError, ErrorKind, MultiError};
use itertools::Itertools;
use mongodb::{
    bson::{self, extjson},
    error::{CommandError, Error as DriverError},
};
use regex::Regex;
use thiserror::Error;
use user_facing_errors::query_engine::DatabaseConstraint;

#[derive(Debug, Error)]
pub enum MongoError {
    #[error("Unsupported MongoDB feature: {0}.")]
    Unsupported(String),

    /// Generic conversion error.
    #[error("Failed to convert '{}' to '{}'.", from, to)]
    ConversionError { from: String, to: String },

    /// Unhandled behavior error.
    #[error("Unhandled behavior: {0}.")]
    UnhandledError(String),

    /// ObjectID specific conversion error.
    #[error("Malformed ObjectID: {0}.")]
    MalformedObjectId(String),

    /// Raw Mongo driver error
    #[error("{0}")]
    DriverError(#[from] DriverError),

    #[error("{0}")]
    UuidError(#[from] uuid::Error),

    #[error("{0}")]
    JsonError(#[from] serde_json::Error),

    #[error("{0}")]
    BsonDeserializationError(#[from] bson::de::Error),

    #[error("Missing required argument: '{}'.", argument)]
    MissingRequiredArgumentError { argument: String },

    #[error("Argument type mismatch for '{}'. Have: {:?}, want: {}.", argument, have, want)]
    ArgumentTypeMismatchError {
        argument: String,
        have: String,
        want: String,
    },
}

impl MongoError {
    pub fn argument_type_mismatch(argument: &str, have: String, want: &str) -> Self {
        Self::ArgumentTypeMismatchError {
            argument: argument.to_string(),
            have,
            want: want.to_string(),
        }
    }

    pub fn into_connector_error(self) -> ConnectorError {
        match self {
            MongoError::Unsupported(feature) => ConnectorError::from_kind(ErrorKind::UnsupportedFeature(feature)),
            MongoError::UnhandledError(reason) => ConnectorError::from_kind(ErrorKind::RawApiError(reason)),
            MongoError::UuidError(err) => ConnectorError::from_kind(ErrorKind::ConversionError(err.into())),
            MongoError::JsonError(err) => ConnectorError::from_kind(ErrorKind::ConversionError(err.into())),
            MongoError::BsonDeserializationError(err) => {
                ConnectorError::from_kind(ErrorKind::ConversionError(err.into()))
            }
            MongoError::MissingRequiredArgumentError { argument } => ConnectorError::from_kind(ErrorKind::RawApiError(
                format!("Missing required argument: '{}'.", argument),
            )),
            MongoError::ArgumentTypeMismatchError { argument, have, want } => {
                ConnectorError::from_kind(ErrorKind::RawApiError(format!(
                    "Argument type mismatch for '{}'. Have: {}, want: {}.",
                    argument, have, want
                )))
            }

            err @ MongoError::ConversionError { .. } => {
                ConnectorError::from_kind(ErrorKind::ConversionError(err.into()))
            }

            err @ MongoError::MalformedObjectId(_) => ConnectorError::from_kind(ErrorKind::ConversionError(err.into())),

            MongoError::DriverError(err) => match err.kind.as_ref() {
                mongodb::error::ErrorKind::InvalidArgument { .. } => {
                    ConnectorError::from_kind(ErrorKind::QueryError(Box::new(err.clone())))
                }
                mongodb::error::ErrorKind::Authentication { message, .. } => {
                    // Todo this mapping is only half correct.
                    ConnectorError::from_kind(ErrorKind::AuthenticationFailed { user: message.clone() })
                }

                // Transaction aborted error.
                mongodb::error::ErrorKind::Command(CommandError { code, message, .. }) if *code == 251 => {
                    ConnectorError::from_kind(ErrorKind::TransactionAborted {
                        message: message.to_owned(),
                    })
                }

                mongodb::error::ErrorKind::Command(CommandError { code, .. }) if *code == 20 => {
                    ConnectorError::from_kind(ErrorKind::MongoReplicaSetRequired)
                }

                mongodb::error::ErrorKind::Write(write_failure) => match write_failure {
                    mongodb::error::WriteFailure::WriteConcernError(concern_error) => match concern_error.code {
                        11000 => ConnectorError::from_kind(unique_violation_error(concern_error.message.as_str())),
                        code => ConnectorError::from_kind(ErrorKind::RawDatabaseError {
                            code: code.to_string(),
                            message: concern_error.message.clone(),
                        }),
                    },

                    mongodb::error::WriteFailure::WriteError(write_error) => match write_error.code {
                        11000 => ConnectorError::from_kind(unique_violation_error(write_error.message.as_str())),
                        code => ConnectorError::from_kind(ErrorKind::RawDatabaseError {
                            code: code.to_string(),
                            message: write_error.message.clone(),
                        }),
                    },

                    _ => ConnectorError::from_kind(ErrorKind::QueryError(Box::new(err.clone()))),
                },

                mongodb::error::ErrorKind::BulkWrite(err) => {
                    let mut errors = match err.write_errors {
                        Some(ref errors) => errors
                            .iter()
                            .map(|err| match err.code {
                                11000 => unique_violation_error(err.message.as_str()),
                                _ => ErrorKind::RawDatabaseError {
                                    code: err.code.to_string(),
                                    message: format!(
                                        "Bulk write error on write index '{}': {}",
                                        err.index, err.message
                                    ),
                                },
                            })
                            .collect_vec(),

                        None => vec![],
                    };

                    if let Some(ref err) = err.write_concern_error {
                        let kind = match err.code {
                            11000 => unique_violation_error(err.message.as_str()),
                            _ => ErrorKind::RawDatabaseError {
                                code: err.code.to_string(),
                                message: format!("Bulk write concern error: {}", err.message),
                            },
                        };

                        errors.push(kind);
                    };

                    if errors.len() == 1 {
                        ConnectorError::from_kind(errors.into_iter().next().unwrap())
                    } else {
                        ConnectorError::from_kind(ErrorKind::MultiError(MultiError { errors }))
                    }
                }

                mongodb::error::ErrorKind::BsonDeserialization(err) => ConnectorError::from_kind(
                    ErrorKind::InternalConversionError(format!("BSON decode error: {}", err)),
                ),

                mongodb::error::ErrorKind::BsonSerialization(err) => ConnectorError::from_kind(
                    ErrorKind::InternalConversionError(format!("BSON encode error: {}", err)),
                ),

                _ => ConnectorError::from_kind(ErrorKind::RawDatabaseError {
                    code: "unknown".to_owned(),
                    message: format!("{}", err),
                }),
            },
        }
    }
}

fn unique_violation_error(message: &str) -> ErrorKind {
    ErrorKind::UniqueConstraintViolation {
        constraint: match parse_unique_index_violation(message) {
            Some(index) => DatabaseConstraint::Index(index),
            None => DatabaseConstraint::CannotParse,
        },
    }
}

fn parse_unique_index_violation(message: &str) -> Option<String> {
    let re = Regex::new(r"duplicate\skey\serror\scollection:\s.*\sindex:\s(.*)\sdup\skey").unwrap();

    match re.captures(message) {
        Some(caps) => caps.get(1).map(|cap| cap.as_str().to_owned()),
        None => None,
    }
}

impl From<mongodb::bson::oid::Error> for MongoError {
    fn from(err: mongodb::bson::oid::Error) -> Self {
        MongoError::MalformedObjectId(format!("{}", err))
    }
}

impl From<extjson::de::Error> for MongoError {
    fn from(err: extjson::de::Error) -> Self {
        match err {
            extjson::de::Error::InvalidObjectId(oid_err) => oid_err.into(),
            extjson::de::Error::DeserializationError { message: _ } => MongoError::ConversionError {
                from: "JSON".to_string(),
                to: "BSON".to_string(),
            },
            // Needed because `extjson::de::Error` is flagged as #[non_exhaustive]
            err => MongoError::UnhandledError(format!("{:?}", err)),
        }
    }
}

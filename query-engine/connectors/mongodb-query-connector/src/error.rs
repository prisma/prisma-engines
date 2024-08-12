use connector_interface::error::{ConnectorError, ErrorKind, MultiError};
use itertools::Itertools;
use mongodb::{
    bson::{self, extjson},
    error::{CommandError, Error as DriverError, TRANSIENT_TRANSACTION_ERROR},
};
use query_structure::{CompositeFieldRef, Field, ScalarFieldRef, SelectedField, VirtualSelection};
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

    // Unhandled conversion error. Mostly used for `#[non-exhaustive]` flagged Mongo errors.
    #[error("Unhandled conversion error: {0}.")]
    UnhandledConversionError(anyhow::Error),
    /// Conversion error related to a datamodel field.
    #[error("Failed to convert '{}' to '{}' for the field '{}'.", from, to, field_name)]
    FieldConversionError {
        field_name: String,
        from: String,
        to: String,
    },

    /// ObjectID specific conversion error.
    #[error("Malformed ObjectID: {0}.")]
    MalformedObjectId(String),

    /// ObjectID specific conversion error related to a datamodel field.
    #[error("Malformed ObjectID: {} for the field '{}'.", reason, field_name)]
    FieldMalformedObjectId { field_name: String, reason: String },

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

    #[error("Record does not exist: {cause}")]
    RecordDoesNotExist { cause: String },
}

impl MongoError {
    pub fn argument_type_mismatch(argument: &str, have: String, want: &str) -> Self {
        Self::ArgumentTypeMismatchError {
            argument: argument.to_string(),
            have,
            want: want.to_string(),
        }
    }

    pub fn decorate_with_field_name(self, field_name: &str) -> Self {
        let field_name = field_name.to_owned();

        match self {
            MongoError::ConversionError { from, to } => MongoError::FieldConversionError { field_name, from, to },
            MongoError::MalformedObjectId(oid) => MongoError::FieldMalformedObjectId {
                field_name,
                reason: oid,
            },
            err => err,
        }
    }
}

impl MongoError {
    pub fn into_connector_error(self) -> ConnectorError {
        match self {
            MongoError::Unsupported(feature) => ConnectorError::from_kind(ErrorKind::UnsupportedFeature(feature)),
            MongoError::UnhandledConversionError(err) => ConnectorError::from_kind(ErrorKind::ConversionError(err)),
            MongoError::UuidError(err) => ConnectorError::from_kind(ErrorKind::ConversionError(err.into())),
            MongoError::JsonError(err) => ConnectorError::from_kind(ErrorKind::ConversionError(err.into())),
            MongoError::BsonDeserializationError(err) => {
                ConnectorError::from_kind(ErrorKind::ConversionError(err.into()))
            }
            MongoError::MissingRequiredArgumentError { argument } => ConnectorError::from_kind(ErrorKind::RawApiError(
                format!("Missing required argument: '{argument}'."),
            )),
            MongoError::ArgumentTypeMismatchError { argument, have, want } => {
                ConnectorError::from_kind(ErrorKind::RawApiError(format!(
                    "Argument type mismatch for '{argument}'. Have: {have}, want: {want}."
                )))
            }

            err @ MongoError::ConversionError { .. } => {
                ConnectorError::from_kind(ErrorKind::ConversionError(err.into()))
            }

            err @ MongoError::FieldConversionError { .. } => {
                ConnectorError::from_kind(ErrorKind::ConversionError(err.into()))
            }

            err @ MongoError::MalformedObjectId(_) => ConnectorError::from_kind(ErrorKind::ConversionError(err.into())),

            err @ MongoError::FieldMalformedObjectId { .. } => {
                ConnectorError::from_kind(ErrorKind::ConversionError(err.into()))
            }

            MongoError::DriverError(err) => {
                let is_transient = err.contains_label(TRANSIENT_TRANSACTION_ERROR);
                let mut conn_err = driver_error_to_connector_error(err);
                conn_err.set_transient(is_transient);

                conn_err
            }

            MongoError::RecordDoesNotExist { cause } => {
                ConnectorError::from_kind(ErrorKind::RecordDoesNotExist { cause })
            }
        }
    }
}

fn driver_error_to_connector_error(err: DriverError) -> ConnectorError {
    match err.kind.as_ref() {
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

        mongodb::error::ErrorKind::Command(CommandError { code, .. }) if *code == 112 => {
            ConnectorError::from_kind(ErrorKind::TransactionWriteConflict)
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
            let errors = err
                .write_errors
                .iter()
                .map(|(index, err)| match err.code {
                    11000 => unique_violation_error(err.message.as_str()),
                    _ => ErrorKind::RawDatabaseError {
                        code: err.code.to_string(),
                        message: format!("Bulk write error on write index '{}': {}", index, err.message),
                    },
                })
                .chain(err.write_concern_errors.iter().map(|err| match err.code {
                    11000 => unique_violation_error(err.message.as_str()),
                    _ => ErrorKind::RawDatabaseError {
                        code: err.code.to_string(),
                        message: format!("Bulk write concern error: {}", err.message),
                    },
                }))
                .collect_vec();

            if errors.len() == 1 {
                ConnectorError::from_kind(errors.into_iter().next().unwrap())
            } else {
                ConnectorError::from_kind(ErrorKind::MultiError(MultiError { errors }))
            }
        }

        mongodb::error::ErrorKind::BsonDeserialization(err) => {
            ConnectorError::from_kind(ErrorKind::InternalConversionError(format!("BSON decode error: {err}")))
        }

        mongodb::error::ErrorKind::BsonSerialization(err) => {
            ConnectorError::from_kind(ErrorKind::InternalConversionError(format!("BSON encode error: {err}")))
        }

        _ => ConnectorError::from_kind(ErrorKind::RawDatabaseError {
            code: "unknown".to_owned(),
            message: format!("{err}"),
        }),
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
        MongoError::MalformedObjectId(format!("{err}"))
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
            err => MongoError::UnhandledConversionError(err.into()),
        }
    }
}

pub trait DecorateErrorWithFieldInformationExtension {
    fn decorate_with_field_info(self, field: &Field) -> Self;
    fn decorate_with_selected_field_info(self, selected_field: &SelectedField) -> Self;
    fn decorate_with_scalar_field_info(self, sf: &ScalarFieldRef) -> Self;
    fn decorate_with_field_name(self, field_name: &str) -> Self;
    fn decorate_with_composite_field_info(self, cf: &CompositeFieldRef) -> Self;
    fn decorate_with_virtual_field_info(self, vs: &VirtualSelection) -> Self;
}

impl<T> DecorateErrorWithFieldInformationExtension for crate::Result<T> {
    fn decorate_with_field_info(self, field: &Field) -> Self {
        self.map_err(|err| err.decorate_with_field_name(field.name()))
    }

    fn decorate_with_selected_field_info(self, selected_field: &SelectedField) -> Self {
        match selected_field {
            SelectedField::Scalar(sf) => self.decorate_with_scalar_field_info(sf),
            SelectedField::Composite(composite_sel) => self.decorate_with_composite_field_info(&composite_sel.field),
            SelectedField::Relation(_) => unreachable!(),
            SelectedField::Virtual(vs) => self.decorate_with_virtual_field_info(vs),
        }
    }

    fn decorate_with_scalar_field_info(self, sf: &ScalarFieldRef) -> Self {
        self.map_err(|err| err.decorate_with_field_name(sf.name()))
    }

    fn decorate_with_field_name(self, field_name: &str) -> Self {
        self.map_err(|err| err.decorate_with_field_name(field_name))
    }

    fn decorate_with_composite_field_info(self, cf: &CompositeFieldRef) -> Self {
        self.map_err(|err| err.decorate_with_field_name(cf.name()))
    }

    fn decorate_with_virtual_field_info(self, vs: &VirtualSelection) -> Self {
        self.map_err(|err| err.decorate_with_field_name(&vs.db_alias()))
    }
}

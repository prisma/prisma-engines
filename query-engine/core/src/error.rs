use crate::{
    InterpreterError, QueryGraphBuilderError, QueryGraphError, QueryParserError, QueryParserErrorKind,
    RelationViolation, TransactionError,
};
use bigdecimal::BigDecimal;
use connector::error::ConnectorError;
use prisma_models::DomainError;
use thiserror::Error;

#[derive(Debug, Error)]
#[error(
    "Error converting field \"{field}\" of expected non-nullable type \"{expected_type}\", found incompatible value of \"{found}\"."
)]

pub struct FieldConversionError {
    pub field: String,
    pub expected_type: String,
    pub found: String,
}

impl FieldConversionError {
    pub fn create(field: String, expected_type: String, found: String) -> CoreError {
        CoreError::FieldConversionError(Self {
            field,
            expected_type,
            found,
        })
    }
}

// TODO: Cleanup unused errors after refactorings.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Error in query graph construction: {:?}", _0)]
    QueryGraphError(QueryGraphError),

    #[error("Error in query graph construction: {:?}", _0)]
    QueryGraphBuilderError(QueryGraphBuilderError),

    #[error("Error in connector: {}", _0)]
    ConnectorError(ConnectorError),

    #[error("Error in domain logic: {}", _0)]
    DomainError(DomainError),

    #[error("{0}")]
    QueryParserError(QueryParserError),

    #[error("Unsupported feature: {}", _0)]
    UnsupportedFeatureError(String),

    #[error("Unable to convert {from_type} value \"{value}\" to type {to_type}")]
    ConversionError {
        value: String,
        from_type: String,
        to_type: String,
    },

    #[error("{}", _0)]
    SerializationError(String),

    #[error("{}", _0)]
    InterpreterError(InterpreterError),

    #[error("{}", _0)]
    ConfigurationError(String),

    #[error("{}", _0)]
    TransactionError(#[from] TransactionError),

    #[error("{}", _0)]
    FieldConversionError(#[from] FieldConversionError),
}

impl CoreError {
    pub fn null_serialization_error(field_name: &str) -> Self {
        CoreError::SerializationError(format!(
            "Inconsistent query result: Field {} is required to return data, got `null` instead.",
            field_name
        ))
    }

    pub fn decimal_conversion_error(decimal: &BigDecimal, to_type: &str) -> Self {
        CoreError::ConversionError {
            value: decimal.to_string(),
            from_type: "BigDecimal".into(),
            to_type: to_type.into(),
        }
    }
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

impl From<url::ParseError> for CoreError {
    fn from(e: url::ParseError) -> Self {
        Self::ConfigurationError(format!("Error parsing connection string: {}", e))
    }
}

impl From<connection_string::Error> for CoreError {
    fn from(e: connection_string::Error) -> Self {
        Self::ConfigurationError(format!("Error parsing connection string: {}", e))
    }
}

impl From<CoreError> for user_facing_errors::Error {
    fn from(err: CoreError) -> user_facing_errors::Error {
        match err {
            CoreError::TransactionError(err) => {
                user_facing_errors::KnownError::new(user_facing_errors::query_engine::InteractiveTransactionError {
                    error: err.to_string(),
                })
                .into()
            }

            CoreError::ConnectorError(ConnectorError {
                user_facing_error: Some(user_facing_error),
                ..
            })
            | CoreError::InterpreterError(InterpreterError::ConnectorError(ConnectorError {
                user_facing_error: Some(user_facing_error),
                ..
            })) => user_facing_error.into(),

            CoreError::QueryParserError(query_parser_error)
            | CoreError::QueryGraphBuilderError(QueryGraphBuilderError::QueryParserError(query_parser_error)) => {
                let known_error = match query_parser_error.error_kind {
                    QueryParserErrorKind::RequiredValueNotSetError => {
                        user_facing_errors::KnownError::new(user_facing_errors::query_engine::MissingRequiredValue {
                            path: format!("{}", query_parser_error.path),
                        })
                    }
                    _ => user_facing_errors::KnownError::new(user_facing_errors::query_engine::QueryValidationFailed {
                        query_validation_error: format!("{}", query_parser_error.error_kind),
                        query_position: format!("{}", query_parser_error.path),
                    }),
                };

                known_error.into()
            }

            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::MissingRequiredArgument {
                argument_name,
                object_name,
                field_name,
            }) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::MissingRequiredArgument {
                argument_name,
                field_name,
                object_name,
            })
            .into(),

            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::RelationViolation(RelationViolation {
                relation_name,
                model_a_name,
                model_b_name,
            }))
            | CoreError::InterpreterError(InterpreterError::QueryGraphBuilderError(
                QueryGraphBuilderError::RelationViolation(RelationViolation {
                    relation_name,
                    model_a_name,
                    model_b_name,
                }),
            )) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::RelationViolation {
                relation_name,
                model_a_name,
                model_b_name,
            })
            .into(),

            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::RecordNotFound(details))
            | CoreError::InterpreterError(InterpreterError::QueryGraphBuilderError(
                QueryGraphBuilderError::RecordNotFound(details),
            )) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::ConnectedRecordsNotFound {
                details,
            })
            .into(),

            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::InputError(details)) => {
                user_facing_errors::KnownError::new(user_facing_errors::query_engine::InputError { details }).into()
            }

            CoreError::InterpreterError(InterpreterError::InterpretationError(msg, Some(cause))) => {
                match cause.as_ref() {
                    InterpreterError::QueryGraphBuilderError(QueryGraphBuilderError::RecordNotFound(cause)) => {
                        user_facing_errors::KnownError::new(
                            user_facing_errors::query_engine::RecordRequiredButNotFound { cause: cause.clone() },
                        )
                        .into()
                    }
                    InterpreterError::QueryGraphBuilderError(QueryGraphBuilderError::RelationViolation(
                        RelationViolation {
                            relation_name,
                            model_a_name,
                            model_b_name,
                        },
                    )) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::RelationViolation {
                        model_a_name: model_a_name.clone(),
                        model_b_name: model_b_name.clone(),
                        relation_name: relation_name.clone(),
                    })
                    .into(),
                    InterpreterError::QueryGraphBuilderError(QueryGraphBuilderError::RecordsNotConnected {
                        relation_name,
                        parent_name,
                        child_name,
                    }) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::RecordsNotConnected {
                        parent_name: parent_name.clone(),
                        child_name: child_name.clone(),
                        relation_name: relation_name.clone(),
                    })
                    .into(),
                    _ => user_facing_errors::KnownError::new(user_facing_errors::query_engine::InterpretationError {
                        details: format!("{}: {}", msg, cause),
                    })
                    .into(),
                }
            }

            CoreError::FieldConversionError(FieldConversionError {
                field,
                expected_type,
                found,
            }) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::MissingFieldsInModel {
                field,
                expected_type,
                found,
            })
            .into(),

            CoreError::ConversionError { .. } => {
                user_facing_errors::KnownError::new(user_facing_errors::query_engine::ValueOutOfRange {
                    details: err.to_string(),
                })
                .into()
            }

            _ => user_facing_errors::Error::from_dyn_error(&err),
        }
    }
}

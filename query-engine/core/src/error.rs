use crate::{
    InterpreterError, QueryGraphBuilderError, QueryGraphError, QueryParserError, QueryParserErrorKind,
    RelationViolation,
};
use connector::error::ConnectorError;
use prisma_models::DomainError;
use thiserror::Error;

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

    #[error("{}", _0)]
    ConversionError(String),

    #[error("{}", _0)]
    SerializationError(String),

    #[error("{}", _0)]
    InterpreterError(InterpreterError),
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

impl From<CoreError> for user_facing_errors::Error {
    fn from(err: CoreError) -> user_facing_errors::Error {
        match err {
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
                model_a_name,
                model_b_name,
                relation_name,
            }))
            | CoreError::InterpreterError(InterpreterError::QueryGraphBuilderError(
                QueryGraphBuilderError::RelationViolation(RelationViolation {
                    model_a_name,
                    model_b_name,
                    relation_name,
                }),
            )) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::RelationViolation {
                model_a_name,
                model_b_name,
                relation_name,
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
                    InterpreterError::QueryGraphBuilderError(QueryGraphBuilderError::RelationViolation(
                        RelationViolation {
                            model_a_name,
                            model_b_name,
                            relation_name,
                        },
                    )) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::RelationViolation {
                        model_a_name: model_a_name.clone(),
                        model_b_name: model_b_name.clone(),
                        relation_name: relation_name.clone(),
                    })
                    .into(),
                    InterpreterError::QueryGraphBuilderError(QueryGraphBuilderError::RecordsNotConnected {
                        parent_name,
                        child_name,
                        relation_name,
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
            _ => user_facing_errors::Error::from_dyn_error(&err),
        }
    }
}

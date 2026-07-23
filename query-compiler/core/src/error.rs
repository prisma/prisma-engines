use crate::{QueryGraphBuilderError, RelationViolation};
use indexmap::IndexMap;
use query_structure::DomainError;
use thiserror::Error;
use user_facing_errors::UnknownError;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Error in query graph construction: {:?}", _0)]
    QueryGraphBuilderError(QueryGraphBuilderError),

    #[error("Error in domain logic: {}", _0)]
    DomainError(DomainError),

    #[error("Failed to parse database version: {}. Reason: {}", version, reason)]
    UnexpectedDatabaseVersion { version: String, reason: String },
}

impl From<QueryGraphBuilderError> for CoreError {
    fn from(e: QueryGraphBuilderError) -> CoreError {
        CoreError::QueryGraphBuilderError(e)
    }
}

impl From<DomainError> for CoreError {
    fn from(e: DomainError) -> CoreError {
        CoreError::DomainError(e)
    }
}

impl From<CoreError> for user_facing_errors::Error {
    fn from(err: CoreError) -> user_facing_errors::Error {
        match err {
            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::QueryParserError(err)) => {
                user_facing_errors::Error::from(err)
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
                relation,
                model_a: model_a_name,
                model_b: model_b_name,
            })) => user_facing_errors::KnownError::new(user_facing_errors::query_engine::RelationViolation {
                relation_name: relation,
                model_a_name,
                model_b_name,
            })
            .into(),

            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::RecordNotFound(details)) => {
                user_facing_errors::KnownError::new(user_facing_errors::query_engine::ConnectedRecordsNotFound {
                    details,
                })
                .into()
            }

            CoreError::QueryGraphBuilderError(QueryGraphBuilderError::InputError(details)) => {
                user_facing_errors::KnownError::new(user_facing_errors::query_engine::InputError { details }).into()
            }

            _ => UnknownError::new(&err).into(),
        }
    }
}

#[derive(Debug, serde::Serialize, PartialEq)]
pub struct ExtendedUserFacingError {
    #[serde(flatten)]
    user_facing_error: user_facing_errors::Error,

    #[serde(skip_serializing_if = "IndexMap::is_empty")]
    extensions: IndexMap<String, serde_json::Value>,
}

impl ExtendedUserFacingError {
    pub fn set_extension(&mut self, key: String, val: serde_json::Value) {
        self.extensions.entry(key).or_insert(val);
    }
}

impl From<CoreError> for ExtendedUserFacingError {
    fn from(error: CoreError) -> Self {
        ExtendedUserFacingError {
            user_facing_error: error.into(),
            extensions: Default::default(),
        }
    }
}

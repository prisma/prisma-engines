use crate::{QueryGraphError, QueryParserError};
use prisma_models::{DomainError, RelationFieldRef};

#[derive(Debug)]
pub enum QueryGraphBuilderError {
    /// Logic error in the construction of the schema.
    /// Not a user error.
    SchemaError(String),

    /// User input error that was't (and can't) be caught
    /// by the general validation during query document parsing.
    InputError(String),

    /// More specific input error for when an argument is missing for a field on a specific model.
    MissingRequiredArgument {
        argument_name: String,
        field_name: String,
        object_name: String,
    },

    /// Wraps the initial parsing stage errors.
    QueryParserError(QueryParserError),

    /// Wraps transformation errors from the prisma models.
    DomainError(DomainError),

    /// General assertion error.
    AssertionError(String),

    RelationViolation(RelationViolation),

    RecordsNotConnected {
        relation_name: String,
        parent_name: String,
        // parent_where: Option<Box<RecordFinderInfo>>,
        child_name: String,
        // child_where: Option<Box<RecordFinderInfo>>,
    },

    RecordNotFound(String),

    QueryGraphError(QueryGraphError),
}

#[derive(Debug)]
pub struct RelationViolation {
    pub(crate) relation_name: String,
    pub(crate) model_a_name: String,
    pub(crate) model_b_name: String,
}

impl From<RelationFieldRef> for RelationViolation {
    fn from(rf: RelationFieldRef) -> Self {
        Self::from(&rf)
    }
}

impl From<&RelationFieldRef> for RelationViolation {
    fn from(rf: &RelationFieldRef) -> Self {
        let relation = rf.relation();

        Self {
            relation_name: relation.name.clone(),
            model_a_name: relation.model_a().name.clone(),
            model_b_name: relation.model_b().name.clone(),
        }
    }
}

impl From<DomainError> for QueryGraphBuilderError {
    fn from(err: DomainError) -> Self {
        QueryGraphBuilderError::DomainError(err)
    }
}

impl From<QueryParserError> for QueryGraphBuilderError {
    fn from(err: QueryParserError) -> Self {
        QueryGraphBuilderError::QueryParserError(err)
    }
}

impl From<QueryGraphError> for QueryGraphBuilderError {
    fn from(err: QueryGraphError) -> Self {
        QueryGraphBuilderError::QueryGraphError(err)
    }
}

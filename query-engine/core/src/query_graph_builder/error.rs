// use std::fmt;
use crate::QueryParserError;
use prisma_models::{DomainError, RelationFieldRef};

#[derive(Debug)]
pub enum QueryGraphBuilderError {
    /// Logic error in the construction of the schema.
    /// Not a user error.
    SchemaError(String),

    /// User input error that was't (and can't) be caught
    /// by the general validation during query document parsing.
    InputError(String),

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
}

#[derive(Debug)]
pub struct RelationViolation {
    relation_name: String,
    model_a_name: String,
    model_b_name: String,
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

// impl fmt::Display for QueryValidationError {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         write!(
//             f,
//             "Error occurred during query validation & transformation:\n{}",
//             self.format(0)
//         )
//     }
// }

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

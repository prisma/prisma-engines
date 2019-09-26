// use std::fmt;
use crate::QueryParserError;
use prisma_models::DomainError;

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

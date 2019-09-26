use crate::{query_document::QueryValue, schema::InputType};
use std::fmt;

#[derive(Debug)]
pub enum QueryParserError {
    AssertionError(String),
    RequiredValueNotSetError,
    FieldNotFoundError,
    ArgumentNotFoundError,
    AtLeastOneSelectionError,
    ValueParseError(String),
    // InputFieldValidationError,
    ValueTypeMismatchError {
        have: QueryValue,
        want: InputType,
    },
    FieldValidationError {
        field_name: String,
        inner: Box<QueryParserError>,
    },
    ArgumentValidationError {
        argument: String,
        inner: Box<QueryParserError>,
    },
    ObjectValidationError {
        object_name: String,
        inner: Box<QueryParserError>,
    },
}

impl QueryParserError {
    pub fn format(&self, ident: usize) -> String {
        match self {
            QueryParserError::AssertionError(reason) => format!("General assertion error: {}.", reason),
            QueryParserError::RequiredValueNotSetError => "A value is required but not set.".into(),
            QueryParserError::FieldNotFoundError => "Field does not exist on enclosing type.".into(),
            QueryParserError::ArgumentNotFoundError => "Argument does not exist on enclosing type.".into(),
            QueryParserError::AtLeastOneSelectionError => "At least one selection is required.".into(),
            QueryParserError::ValueParseError(reason) => format!("Error parsing value: {}.", reason),
            QueryParserError::ValueTypeMismatchError { have, want } => {
                format!("Value types mismatch. Have: {:?}, want: {:?}", have, want)
            } // wip value/type formatting

            // Validation root
            QueryParserError::ObjectValidationError { object_name, inner } => format!(
                "{} (object)\n{}",
                object_name,
                Self::ident(inner.format(ident + 2), ident + 2)
            ),

            QueryParserError::FieldValidationError { field_name, inner } => format!(
                "{} (field)\n{}",
                field_name,
                Self::ident(inner.format(ident + 2), ident + 2)
            ),
            QueryParserError::ArgumentValidationError { argument, inner } => format!(
                "{} (argument)\n{}",
                argument,
                Self::ident(inner.format(ident + 2), ident + 2)
            ),
        }
    }

    fn ident(s: String, size: usize) -> String {
        format!("{}↳ {}", " ".repeat(size), s)
    }
}

impl fmt::Display for QueryParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Error occurred during query validation & transformation:\n{}",
            self.format(0)
        )
    }
}

impl From<prisma_models::DomainError> for QueryParserError {
    fn from(err: prisma_models::DomainError) -> Self {
        QueryParserError::AssertionError(format!("Domain error occurred: {}", err))
    }
}

use serde::Serialize;
use std::fmt;
use user_facing_error_macros::*;

#[derive(Debug, PartialEq, Eq, Serialize, Clone)]
#[serde(untagged)]
pub enum DatabaseConstraint {
    Fields(Vec<String>),
    Index(String),
    ForeignKey,
}

impl fmt::Display for DatabaseConstraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fields(fields) => {
                let quoted_fields: Vec<String> = fields.iter().map(|f| format!("`{}`", f)).collect();
                write!(f, "fields: ({})", quoted_fields.join(","))
            }
            Self::Index(index) => write!(f, "constraint: `{}`", index),
            Self::ForeignKey => write!(f, "foreign key"),
        }
    }
}

impl From<Vec<String>> for DatabaseConstraint {
    fn from(fields: Vec<String>) -> Self {
        Self::Fields(fields)
    }
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2000",
    message = "The provided value for the column is too long for the column's type. Column: ${column_name}"
)]
pub struct InputValueTooLong {
    pub column_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2001",
    message = "The record searched for in the where condition (`${model_name}.${argument_name} = ${argument_value}`) does not exist"
)]
pub struct RecordNotFound {
    /// Model name from Prisma schema
    pub model_name: String,

    /// Argument name from a supported query on a Prisma schema model
    pub argument_name: String,

    /// Concrete value provided for an argument on a query. Should be peeked/truncated if too long
    /// to display in the error message
    pub argument_value: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2002", message = "Unique constraint failed on the ${constraint}")]
pub struct UniqueKeyViolation {
    /// Field name from one model from Prisma schema
    #[serde(rename = "target")]
    pub constraint: DatabaseConstraint,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2003",
    message = "Foreign key constraint failed on the field: `${field_name}`"
)]
pub struct ForeignKeyViolation {
    /// Field name from one model from Prisma schema
    pub field_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2004", message = "A constraint failed on the database: `${database_error}`")]
pub struct ConstraintViolation {
    /// Database error returned by the underlying data source
    pub database_error: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2005",
    message = "The value `${field_value}` stored in the database for the field `${field_name}` is invalid for the field's type"
)]
pub struct StoredValueIsInvalid {
    /// Concrete value provided for a field on a model in Prisma schema. Should be peeked/truncated if too long to display in the error message
    pub field_value: String,

    /// Field name from one model from Prisma schema
    pub field_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2006",
    message = "The provided value `${field_value}` for `${model_name}` field `${field_name}` is not valid"
)]
pub struct TypeMismatch {
    /// Concrete value provided for a field on a model in Prisma schema. Should be peeked/truncated if too long to display in the error message
    pub field_value: String,

    /// Model name from Prisma schema
    pub model_name: String,

    /// Field name from one model from Prisma schema
    pub field_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2007", message = "Data validation error `${database_error}`")]
pub struct TypeMismatchInvalidCustomType {
    /// Database error returned by the underlying data source
    pub database_error: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2008",
    message = "Failed to parse the query `${query_parsing_error}` at `${query_position}`"
)]
pub struct QueryParsingFailed {
    /// Error(s) encountered when trying to parse a query in the query engine
    pub query_parsing_error: String,

    /// Location of the incorrect parsing, validation in a query. Represented by tuple or object with (line, character)
    pub query_position: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2009",
    message = "Failed to validate the query `${query_validation_error}` at `${query_position}`"
)]
pub struct QueryValidationFailed {
    /// Error(s) encountered when trying to validate a query in the query engine
    pub query_validation_error: String,

    /// Location of the incorrect parsing, validation in a query. Represented by tuple or object with (line, character)
    pub query_position: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2010", message = "Raw query failed. Code: `${code}`. Message: `${message}`")]
pub struct RawQueryFailed {
    pub code: String,
    pub message: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2011", message = "Null constraint violation on the ${constraint}")]
pub struct NullConstraintViolation {
    pub constraint: DatabaseConstraint,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2012", message = "Missing a required value at `${path}`")]
pub struct MissingRequiredValue {
    pub path: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2013",
    message = "Missing the required argument `${argument_name}` for field `${field_name}` on `${object_name}`."
)]
pub struct MissingRequiredArgument {
    pub argument_name: String,
    pub field_name: String,
    pub object_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2014",
    message = "The change you are trying to make would violate the required relation '${relation_name}' between the `${model_a_name}` and `${model_b_name}` models."
)]
pub struct RelationViolation {
    pub relation_name: String,
    pub model_a_name: String,
    pub model_b_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2015", message = "A related record could not be found. ${details}")]
pub struct RelatedRecordNotFound {
    pub details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2016", message = "Query interpretation error. ${details}")]
pub struct InterpretationError {
    pub details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2017",
    message = "The records for relation `${relation_name}` between the `${parent_name}` and `${child_name}` models are not connected."
)]
pub struct RecordsNotConnected {
    pub relation_name: String,
    pub parent_name: String,
    pub child_name: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2018",
    message = "The required connected records were not found. ${details}"
)]
pub struct ConnectedRecordsNotFound {
    pub details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2019", message = "Input error. ${details}")]
pub struct InputError {
    pub details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(code = "P2020", message = "Value out of range for the type. ${details}")]
pub struct ValueOutOfRange {
    pub details: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2021",
    message = "The table `${table}` does not exist in the current database."
)]
pub struct TableDoesNotExist {
    pub table: String,
}

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2022",
    message = "The column `${column}` does not exist in the current database."
)]
pub struct ColumnDoesNotExist {
    pub column: String,
}

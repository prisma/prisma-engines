use serde::Serialize;
use serde_json::json;
use user_facing_error_macros::*;

#[derive(Debug, UserFacingError, Serialize)]
#[user_facing(
    code = "P2009",
    message = "Failed to validate the query: `{query_validation_error}` at `{query_position}`"
)]
pub struct LegacyQueryValidationFailed {
    /// Error(s) encountered when trying to validate a query in the query engine
    pub query_validation_error: String,

    /// Location of the incorrect parsing, validation in a query. Represented by tuple or object with (line, character)
    pub query_position: String,
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    kind: ValidationErrorKind,
    #[serde(skip)]
    message: String,
    path: Vec<String>,
    #[serde(flatten)]
    meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub enum ValidationErrorKind {
    EmptySelection,
}

impl crate::UserFacingError for ValidationError {
    const ERROR_CODE: &'static str = "P2009";

    fn message(&self) -> String {
        self.message.clone()
    }
}

impl ValidationError {
    pub fn empty_selection(path: Vec<String>, o: OutputTypeDescription) -> Self {
        let message = String::from("Expected a minimum of 1 field, found 0");
        ValidationError {
            kind: ValidationErrorKind::EmptySelection,
            meta: o.into(),
            message,
            path,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct OutputTypeDescription {
    name: String,
    fields: Vec<OutputTypeDescriptionField>,
}

impl OutputTypeDescription {
    pub fn new(name: String, fields: Vec<OutputTypeDescriptionField>) -> Self {
        OutputTypeDescription { name, fields }
    }
}

impl From<OutputTypeDescription> for Option<serde_json::Value> {
    fn from(o: OutputTypeDescription) -> Self {
        Some(json!({ "outputType": o }))
    }
}

#[derive(Debug, Serialize)]
pub struct OutputTypeDescriptionField {
    name: String,
    type_name: String,
    is_relation: bool,
}

impl OutputTypeDescriptionField {
    pub fn new(name: String, type_name: String, is_relation: bool) -> Self {
        OutputTypeDescriptionField {
            name,
            type_name,
            is_relation,
        }
    }
}
#[derive(Debug, Serialize)]
pub struct InputTypeDescription {
    name: String,
    fields: Vec<InputTypeDescriptionField>,
}

#[derive(Debug, Serialize)]
pub struct InputTypeDescriptionField {
    name: String,
    type_names: Vec<String>,
    is_relation: bool,
}

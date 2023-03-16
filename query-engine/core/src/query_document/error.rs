use std::fmt;
use user_facing_errors::query_engine::validation::{self, ValidationError};

pub(crate) mod conversions {
    use super::*;
    use crate::schema::{InputType, OutputType, ScalarType};

    /// converts an schema object to the narrower validation::OutputTypeDescription
    /// representation of an output field that is part of a validation error information.
    pub(crate) fn schema_object_to_output_type_description(
        o: &schema::ObjectTypeStrongRef,
    ) -> validation::OutputTypeDescription {
        let name = o.identifier.name().to_owned();
        let fields: Vec<validation::OutputTypeDescriptionField> = o
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name.to_owned();
                let type_name = to_simplified_output_type_name(field.field_type.as_ref());
                let is_relation = field.field_type.is_relation();

                validation::OutputTypeDescriptionField::new(name, type_name, is_relation)
            })
            .collect();
        validation::OutputTypeDescription::new(name, fields)
    }

    pub(crate) fn schema_input_object_type_to_input_type_description(
        i: &schema::InputObjectTypeStrongRef,
    ) -> validation::InputTypeDescription {
        let name = i.identifier.to_string();
        let fields: Vec<validation::InputTypeDescriptionField> = i
            .get_fields()
            .iter()
            .map(|field| {
                let name = field.name.clone();
                let type_names: Vec<String> = field
                    .field_types
                    .iter()
                    .map(|typ| to_simplified_input_type_name(typ))
                    .collect();
                validation::InputTypeDescriptionField::new(name, type_names, field.is_required)
            })
            .collect();
        validation::InputTypeDescription::new(name, fields)
    }

    pub(crate) fn schema_output_field_to_input_type_description(
        o: &schema::OutputFieldRef,
    ) -> validation::InputTypeDescription {
        let name = o.name.clone();

        let fields = o
            .arguments
            .iter()
            .map(|field| {
                let name = field.name.clone();
                let type_names: Vec<String> = field
                    .field_types
                    .iter()
                    .map(|typ| to_simplified_input_type_name(typ))
                    .collect();
                validation::InputTypeDescriptionField::new(name, type_names, field.is_required)
            })
            .collect();

        validation::InputTypeDescription::new(name, fields)
    }

    pub(crate) fn schema_arguments_to_argument_description_vec(
        arguments: &[schema::InputFieldRef],
    ) -> Vec<validation::ArgumentDescription> {
        arguments
            .iter()
            .map(|input_field_ref| {
                validation::ArgumentDescription::new(
                    input_field_ref.name.to_string(),
                    input_field_ref
                        .field_types
                        .iter()
                        .map(|typ| to_simplified_input_type_name(typ))
                        .collect(),
                )
            })
            .collect::<Vec<_>>()
    }

    pub(crate) fn scalar_type_to_argument_description(
        arg_name: String,
        scalar_type: &ScalarType,
    ) -> validation::ArgumentDescription {
        validation::ArgumentDescription::new(arg_name, vec![scalar_type.to_string()])
    }

    pub(crate) fn input_type_to_argument_description(
        arg_name: String,
        input_type: &InputType,
    ) -> validation::ArgumentDescription {
        validation::ArgumentDescription::new(arg_name, vec![to_simplified_input_type_name(input_type)])
    }

    pub fn to_simplified_input_type_name(typ: &InputType) -> String {
        match typ {
            InputType::Enum(_) => String::from("enum"),
            InputType::List(o) => format!("{}[]", to_simplified_input_type_name(o.as_ref())),
            InputType::Object(o) => o
                .upgrade()
                .map(|f| f.identifier.name().to_owned())
                .unwrap_or_else(|| String::from("Object")),
            InputType::Scalar(s) => s.to_string(),
        }
    }

    pub fn to_simplified_output_type_name(typ: &OutputType) -> String {
        match typ {
            OutputType::Enum(_) => String::from("enum"),
            OutputType::List(o) => format!("{}[]", to_simplified_output_type_name(o)),
            OutputType::Object(o) => o
                .upgrade()
                .map(|f| f.identifier.name().to_owned())
                .unwrap_or_else(|| String::from("Object")),
            OutputType::Scalar(s) => s.to_string(),
        }
    }
}

#[derive(Debug)]
pub enum QueryParserError {
    New(ValidationError),
    Legacy {
        path: QueryPath,
        error_kind: QueryParserErrorKind,
    },
}

impl QueryParserError {
    // TODO: remove after refactoring and removal of Legacy errors
    pub fn into_user_facing_error(self) -> user_facing_errors::Error {
        match self {
            QueryParserError::New(err) => user_facing_errors::KnownError::from(err).into(),
            _ => todo!(),
        }
    }
}

impl From<ValidationError> for QueryParserError {
    fn from(err: ValidationError) -> Self {
        QueryParserError::New(err)
    }
}

// TODO: remove in favor of derived display on QueryParserError once Legacy is removed
impl fmt::Display for QueryParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Legacy { path, error_kind } => {
                write!(f, "Query parsing/validation error at `{}`: {}", path, error_kind)
            }
            QueryParserError::New(ve) => write!(f, "Query parsing/validation error. Message: {}", ve),
        }
    }
}

pub type SelectionPath = QueryPath;
pub type ArgumentPath = QueryPath;
#[derive(Debug, Clone, Default)]
pub struct QueryPath {
    segments: Vec<String>,
}

impl QueryPath {
    pub fn new(initial_segment: String) -> Self {
        Self {
            segments: vec![initial_segment],
        }
    }

    pub fn add(&self, segment: String) -> Self {
        let mut path = self.clone();
        path.segments.push(segment);
        path
    }

    pub fn last(&self) -> Option<&str> {
        self.segments.last().map(|s| s.as_str())
    }

    pub fn segments(&self) -> Vec<String> {
        self.segments.clone()
    }
}

impl fmt::Display for ArgumentPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.segments.join("."))
    }
}

#[derive(Debug)]
pub enum QueryParserErrorKind {
    InputUnionParseError { parsing_errors: Vec<QueryParserError> },
}

impl fmt::Display for QueryParserErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InputUnionParseError { parsing_errors } => write!(
                f,
                "Unable to match input value to any allowed input type for the field. Parse errors: [{}]",
                parsing_errors
                    .iter()
                    .map(|err| format!("{err}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

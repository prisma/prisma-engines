use crate::{schema::InputType, ArgumentValue};
use fmt::Display;
use itertools::Itertools;
use std::fmt;
use user_facing_errors::query_engine::validation::{self, ValidationError};

pub(crate) mod conversions {
    use super::*;

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
                let type_name = field.field_type.to_string();
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
                let type_names: Vec<String> = field.field_types.iter().map(|typ| typ.to_string()).collect();
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
                    input_field_ref.field_types.iter().map(|typ| typ.to_string()).collect(),
                )
            })
            .collect::<Vec<_>>()
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
            QueryParserError::New(err) => user_facing_errors::Error::from(err),
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
                write!(f, "Query parsing/validation error at `{}`: {}", path, error_kind,)
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct QueryPath {
    pub segments: Vec<String>,
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
}

impl fmt::Display for QueryPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.segments.join("."))
    }
}

#[derive(Debug)]
pub enum QueryParserErrorKind {
    AssertionError(String),
    RequiredValueNotSetError,
    FieldCountError(FieldCountError),
    ValueParseError(String),
    ValueTypeMismatchError { have: ArgumentValue, want: InputType },
    InputUnionParseError { parsing_errors: Vec<QueryParserError> },
    ValueFitError(String),
}

impl Display for QueryParserErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AssertionError(reason) => write!(f, "Assertion error: {reason}."),
            Self::RequiredValueNotSetError => write!(f, "A value is required but not set."),
            Self::FieldCountError(err) => write!(f, "{err}"),
            Self::ValueParseError(reason) => write!(f, "Error parsing value: {reason}."),
            Self::InputUnionParseError { parsing_errors } => write!(
                f,
                "Unable to match input value to any allowed input type for the field. Parse errors: [{}]",
                parsing_errors
                    .iter()
                    .map(|err| format!("{err}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::ValueTypeMismatchError { have, want } => {
                write!(f, "Value types mismatch. Have: {have:?}, want: {want:?}")
            }
            Self::ValueFitError(s) => write!(f, "{s}"),
        }
    }
}

#[derive(Debug)]
pub struct FieldCountError {
    pub min: Option<usize>,
    pub max: Option<usize>,
    pub fields: Option<Vec<String>>,
    pub got: usize,
}

impl FieldCountError {
    pub fn new(min: Option<usize>, max: Option<usize>, fields: Option<Vec<String>>, got: usize) -> Self {
        Self { min, max, fields, got }
    }
}

impl Display for FieldCountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.fields {
            None => match (self.min, self.max) {
                (Some(1), Some(1)) => {
                    write!(f, "Expected exactly one field to be present, got {}.", self.got)
                }
                (Some(min), Some(max)) => write!(
                    f,
                    "Expected a minimum of {} and at most {} fields to be present, got {}.",
                    min, max, self.got
                ),
                (Some(min), None) => write!(
                    f,
                    "Expected a minimum of {} fields to be present, got {}.",
                    min, self.got
                ),
                (None, Some(max)) => write!(f, "Expected at most {} fields to be present, got {}.", max, self.got),
                (None, None) => write!(f, "Expected any selection of fields, got {}.", self.got),
            },
            Some(fields) => match (self.min, self.max) {
                (Some(1), Some(1)) => {
                    write!(
                        f,
                        "Expected exactly one field of ({}) to be present, got {}.",
                        fields.iter().join(", "),
                        self.got
                    )
                }
                (Some(min), Some(max)) => write!(
                    f,
                    "Expected a minimum of {} and at most {} fields of ({}) to be present, got {}.",
                    min,
                    max,
                    fields.iter().join(", "),
                    self.got
                ),
                (Some(min), None) => write!(
                    f,
                    "Expected a minimum of {} fields of ({}) to be present, got {}.",
                    min,
                    fields.iter().join(", "),
                    self.got
                ),
                (None, Some(max)) => write!(
                    f,
                    "Expected at most {} fields of ({}) to be present, got {}.",
                    max,
                    fields.iter().join(", "),
                    self.got
                ),
                (None, None) => write!(f, "Expected any selection of fields, got {}.", self.got),
            },
        }
    }
}

impl From<prisma_models::DomainError> for QueryParserError {
    fn from(err: prisma_models::DomainError) -> Self {
        QueryParserError::Legacy {
            path: QueryPath::default(),
            error_kind: QueryParserErrorKind::AssertionError(format!("Domain error occurred: {err}")),
        }
    }
}

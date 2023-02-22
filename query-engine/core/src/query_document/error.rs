use crate::{schema::InputType, ArgumentValue};
use fmt::Display;
use itertools::Itertools;
use serde::Serialize;
use std::fmt;
use user_facing_errors::UserFacingError;

#[derive(Debug)]
pub enum QueryParserError {
    Structured(StructuredQueryParseError),
    Legacy {
        path: QueryPath,
        error_kind: QueryParserErrorKind,
    },
}

impl QueryParserError {
    /// Create a new instance of `QueryParserError`.
    pub fn new_legacy(path: QueryPath, error_kind: QueryParserErrorKind) -> Self {
        Self::Legacy { path, error_kind }
    }

    // deprecated: This must dissappear as soon as errors transition to the StructuredQueryParserError type
    pub fn path(&self) -> Option<QueryPath> {
        match self {
            Self::Legacy { path, error_kind: _ } => Some(path.clone()),
            Self::Structured(_) => None,
        }
    }

    // deprecated: This must dissappear as soon as errors transition to the StructuredQueryParserError type
    pub fn error_kind(&self) -> Option<&QueryParserErrorKind> {
        match self {
            Self::Legacy { path: _, error_kind } => Some(error_kind),
            _ => None,
        }
    }
}

impl fmt::Display for QueryParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Legacy { path, error_kind } => {
                write!(f, "Query parsing/validation error at `{}`: {}", path, error_kind,)
            }
            Self::Structured(_) => todo!(),
        }
    }
}
#[derive(Debug, Serialize, Clone)]
pub struct StructuredQueryParseError {}

impl UserFacingError for StructuredQueryParseError {
    const ERROR_CODE: &'static str = "P2009";

    fn message(&self) -> String {
        todo!()
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
    FieldNotFoundError,
    ArgumentNotFoundError,
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
            Self::FieldNotFoundError => write!(f, "Field does not exist on enclosing type."),
            Self::ArgumentNotFoundError => write!(f, "Argument does not exist on enclosing type."),
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
        QueryParserError::new_legacy(
            QueryPath::default(),
            QueryParserErrorKind::AssertionError(format!("Domain error occurred: {err}")),
        )
    }
}

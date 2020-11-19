use crate::{query_document::QueryValue, schema::InputType};
use fmt::Display;
use std::fmt;

#[derive(Debug)]
pub struct QueryParserError {
    pub path: QueryPath,
    pub error_kind: QueryParserErrorKind,
}

impl QueryParserError {
    /// Create a new instance of `QueryParserError`.
    pub fn new(path: QueryPath, error_kind: QueryParserErrorKind) -> Self {
        Self { path, error_kind }
    }
}

impl fmt::Display for QueryParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Query parsing/validation error at `{}`: {}",
            self.path, self.error_kind,
        )
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
    ValueTypeMismatchError { have: QueryValue, want: InputType },
    InputUnionParseError { parsing_errors: Vec<QueryParserError> },
}

impl Display for QueryParserErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AssertionError(reason) => write!(f, "Assertion error: {}.", reason),
            Self::RequiredValueNotSetError => write!(f, "A value is required but not set."),
            Self::FieldNotFoundError => write!(f, "Field does not exist on enclosing type."),
            Self::ArgumentNotFoundError => write!(f, "Argument does not exist on enclosing type."),
            Self::FieldCountError(err) => write!(f, "{}", err),
            Self::ValueParseError(reason) => write!(f, "Error parsing value: {}.", reason),
            Self::InputUnionParseError { parsing_errors } => write!(
                f,
                "Unable to match input value to any allowed input type for the field. Parse errors: [{}]",
                parsing_errors
                    .iter()
                    .map(|err| format!("{}", err))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::ValueTypeMismatchError { have, want } => {
                write!(f, "Value types mismatch. Have: {:?}, want: {:?}", have, want)
            }
        }
    }
}

#[derive(Debug)]
pub struct FieldCountError {
    pub min: Option<usize>,
    pub max: Option<usize>,
    pub got: usize,
}

impl FieldCountError {
    pub fn new(min: Option<usize>, max: Option<usize>, got: usize) -> Self {
        Self { min, max, got }
    }
}

impl Display for FieldCountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.min, self.max) {
            (Some(min), Some(max)) if min == 1 && max == 1 => {
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
        }
    }
}

impl From<prisma_models::DomainError> for QueryParserError {
    fn from(err: prisma_models::DomainError) -> Self {
        QueryParserError {
            path: QueryPath::default(),
            error_kind: QueryParserErrorKind::AssertionError(format!("Domain error occurred: {}", err)),
        }
    }
}

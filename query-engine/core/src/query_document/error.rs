use crate::{query_document::QueryValue, schema::InputType};
use fmt::Display;
use std::fmt;

#[derive(Debug)]
pub struct QueryParserError {
    pub path: QueryPath,
    pub error_kind: QueryParserErrorKind,
}

impl fmt::Display for QueryParserError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Query parsing/validation error at `{}`:\n{}",
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
                "Unable to match input value to any allowed input type for the field. Parse errors: {:?}",
                parsing_errors
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

impl QueryParserError {
    // pub(crate) fn location(&self) -> String {
    //     let mut node = self;

    //     std::iter::from_fn(|| match node {
    //         QueryParserError::FieldValidationError {
    //             field_name: name,
    //             inner,
    //         }
    //         | QueryParserError::ArgumentValidationError { argument: name, inner }
    //         | QueryParserError::ObjectValidationError {
    //             object_name: name,
    //             inner,
    //         } => {
    //             node = inner.as_ref();
    //             Some(name)
    //         }
    //         _ => None,
    //     })
    //     .fold(String::with_capacity(32), |mut path, elem| {
    //         path.push_str(".");
    //         path.push_str(elem);
    //         path
    //     })
    // }

    // pub fn format(&self, ident: usize) -> String {
    //     match self {
    //         // Validation leaves
    //         QueryParserError::AssertionError(reason) => format!("Assertion error: {}.", reason),
    //         QueryParserError::RequiredValueNotSetError => "A value is required but not set.".into(),
    //         QueryParserError::FieldNotFoundError => "Field does not exist on enclosing type.".into(),
    //         QueryParserError::ArgumentNotFoundError => "Argument does not exist on enclosing type.".into(),
    //         QueryParserError::FieldCountError(err) => format!("{}", err),
    //         QueryParserError::ValueParseError(reason) => format!("Error parsing value: {}.", reason),
    //         QueryParserError::UnionParseError { parsing_errors } => format!(
    //             "Unable to match input value to any allowed input type for the field. Parse errors: {:?}",
    //             parsing_errors
    //         ),
    //         QueryParserError::ValueTypeMismatchError { have, want } => {
    //             format!("Value types mismatch. Have: {:?}, want: {:?}", have, want)
    //         }
    //     }
    // }

    // pub(crate) fn as_missing_value_error(&self) -> Option<user_facing_errors::query_engine::MissingRequiredValue> {
    //     self.as_missing_value_error_recursive(Vec::new())
    //         .map(|path| user_facing_errors::query_engine::MissingRequiredValue { path: path.join(".") })
    // }

    // fn as_missing_value_error_recursive(&self, mut path: Vec<String>) -> Option<Vec<String>> {
    //     match self {
    //         QueryParserError::RequiredValueNotSetError => Some(path),
    //         QueryParserError::FieldValidationError { inner, field_name } => {
    //             path.push(field_name.clone());
    //             inner.as_missing_value_error_recursive(path)
    //         }
    //         QueryParserError::ObjectValidationError { inner, object_name } => {
    //             path.push(object_name.clone());
    //             inner.as_missing_value_error_recursive(path)
    //         }
    //         QueryParserError::ArgumentValidationError { inner, argument } => {
    //             path.push(argument.clone());
    //             inner.as_missing_value_error_recursive(path)
    //         }
    //         _ => None,
    //     }
    // }
}

impl From<prisma_models::DomainError> for QueryParserError {
    fn from(err: prisma_models::DomainError) -> Self {
        QueryParserError {
            path: QueryPath::default(),
            error_kind: QueryParserErrorKind::AssertionError(format!("Domain error occurred: {}", err)),
        }
    }
}

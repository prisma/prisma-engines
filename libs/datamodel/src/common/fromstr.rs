use crate::ast::Span;
use crate::errors::DatamodelError;

/// FromStr trait that respects span.
pub trait FromStrAndSpan: Sized {
    fn from_str_and_span(s: &str, span: Span) -> Result<Self, DatamodelError>;
}

use crate::ast::Span;
use crate::error::DatamodelError;

/// FromStr trait that respects span.
pub trait FromStrAndSpan: Sized {
    fn from_str_and_span(s: &str, span: Span) -> Result<Self, DatamodelError>;
}

use super::*;
use crate::ast::Span;
use crate::common::FromStrAndSpan;
use crate::error::DatamodelError;

impl<T> FromStrAndSpan for T
where
    T: Parsable,
{
    fn from_str_and_span(s: &str, span: Span) -> Result<Self, DatamodelError> {
        match T::parse(s) {
            Some(x) => Ok(x),
            None => Err(DatamodelError::new_literal_parser_error(T::descriptor(), s, span)),
        }
    }
}

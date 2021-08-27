use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
pub struct JsonExtractLastArrayElem<'a> {
    pub(crate) expr: Box<Expression<'a>>,
}

/// This is an internal function used to help construct the JsonArrayEndsInto Comparable
#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
pub(crate) fn json_extract_last_array_elem<'a, E>(expr: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = JsonExtractLastArrayElem {
        expr: Box::new(expr.into()),
    };

    fun.into()
}

#[derive(Debug, Clone, PartialEq)]
#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
pub struct JsonExtractFirstArrayElem<'a> {
    pub(crate) expr: Box<Expression<'a>>,
}

/// This is an internal function used to help construct the JsonArrayBeginsWith Comparable
#[cfg(all(feature = "json", any(feature = "postgresql", feature = "mysql")))]
pub(crate) fn json_extract_first_array_elem<'a, E>(expr: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = JsonExtractFirstArrayElem {
        expr: Box::new(expr.into()),
    };

    fun.into()
}

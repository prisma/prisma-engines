use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonArrayAgg<'a> {
    pub(crate) expr: Box<Expression<'a>>,
}

/// This is an internal function used to help construct the JsonArrayBeginsWith Comparable
pub fn json_array_agg<'a, E>(expr: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = JsonArrayAgg {
        expr: Box::new(expr.into()),
    };

    fun.into()
}

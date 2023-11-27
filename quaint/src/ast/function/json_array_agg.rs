use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonArrayAgg<'a> {
    pub(crate) expr: Box<Expression<'a>>,
}

/// Builds a JSON array out of a list of values.
pub fn json_array_agg<'a, E>(expr: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = JsonArrayAgg {
        expr: Box::new(expr.into()),
    };

    fun.into()
}

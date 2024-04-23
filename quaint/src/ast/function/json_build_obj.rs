use std::borrow::Cow;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonBuildObject<'a> {
    pub(crate) exprs: Vec<(Cow<'a, str>, Expression<'a>)>,
}

/// Builds a JSON object out of a list of key-value pairs.
pub fn json_build_object<'a>(exprs: Vec<(Cow<'a, str>, Expression<'a>)>) -> Function<'a> {
    let fun = JsonBuildObject { exprs };

    fun.into()
}

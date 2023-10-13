use std::borrow::Cow;

use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonBuildObject<'a> {
    pub(crate) exprs: Vec<(Cow<'a, str>, Expression<'a>)>,
}

/// This is an internal function used to help construct the JsonArrayBeginsWith Comparable
pub fn json_build_object<'a>(exprs: Vec<(Cow<'a, str>, Expression<'a>)>) -> Function<'a> {
    let fun = JsonBuildObject { exprs };

    fun.into()
}

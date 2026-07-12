use std::borrow::Cow;

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Fragment {
    StringChunk {
        chunk: String,
    },
    Parameter,
    #[serde(rename_all = "camelCase")]
    ParameterTuple {
        item_prefix: Cow<'static, str>,
        item_separator: Cow<'static, str>,
        item_suffix: Cow<'static, str>,
    },
    #[serde(rename_all = "camelCase")]
    ParameterTupleList {
        item_prefix: Cow<'static, str>,
        item_separator: Cow<'static, str>,
        item_suffix: Cow<'static, str>,
        group_separator: Cow<'static, str>,
    },
}

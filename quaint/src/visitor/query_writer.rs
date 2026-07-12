use std::borrow::Cow;

use crate::Value;
use query_template::{Fragment, QueryTemplate};

pub(crate) trait QueryWriter {
    fn write_string_chunk(&mut self, value: String);
    fn write_parameter(&mut self);
    fn write_parameter_tuple(
        &mut self,
        item_prefix: impl Into<Cow<'static, str>>,
        item_separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
    );
    fn write_parameter_tuple_list(
        &mut self,
        item_prefix: impl Into<Cow<'static, str>>,
        item_separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
        group_separator: impl Into<Cow<'static, str>>,
    );
}

impl QueryWriter for QueryTemplate<Value<'_>> {
    fn write_string_chunk(&mut self, value: String) {
        match self.fragments.last_mut() {
            Some(Fragment::StringChunk { chunk }) => {
                chunk.push_str(value.as_str());
            }
            _ => {
                self.fragments.push(Fragment::StringChunk { chunk: value });
            }
        }
    }

    fn write_parameter(&mut self) {
        self.fragments.push(Fragment::Parameter);
    }

    fn write_parameter_tuple(
        &mut self,
        item_prefix: impl Into<Cow<'static, str>>,
        item_separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
    ) {
        self.fragments.push(Fragment::ParameterTuple {
            item_prefix: item_prefix.into(),
            item_separator: item_separator.into(),
            item_suffix: item_suffix.into(),
        });
    }

    fn write_parameter_tuple_list(
        &mut self,
        item_prefix: impl Into<Cow<'static, str>>,
        item_separator: impl Into<Cow<'static, str>>,
        item_suffix: impl Into<Cow<'static, str>>,
        group_separator: impl Into<Cow<'static, str>>,
    ) {
        self.fragments.push(Fragment::ParameterTupleList {
            item_prefix: item_prefix.into(),
            item_separator: item_separator.into(),
            item_suffix: item_suffix.into(),
            group_separator: group_separator.into(),
        });
    }
}

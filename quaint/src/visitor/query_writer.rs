use crate::Value;
use query_template::{Fragment, QueryTemplate};
use std::fmt;
use std::fmt::Display;

pub(crate) trait QueryWriter {
    fn write_string_chunk<D: fmt::Display>(&mut self, s: D);
    fn write_parameter(&mut self);
    fn write_parameter_tuple(&mut self);
}

impl QueryWriter for QueryTemplate<Value<'_>> {
    fn write_string_chunk<D: Display>(&mut self, value: D) {
        match self.fragments.last_mut() {
            Some(Fragment::StringChunk(chunk)) => {
                chunk.push_str(&value.to_string());
            }
            _ => {
                let mut chunk = String::with_capacity(4096);
                chunk.push_str(&value.to_string());
                self.fragments.push(Fragment::StringChunk(chunk));
            }
        }
    }

    fn write_parameter(&mut self) {
        self.fragments.push(Fragment::Parameter);
    }

    fn write_parameter_tuple(&mut self) {
        self.fragments.push(Fragment::ParameterTuple);
    }
}

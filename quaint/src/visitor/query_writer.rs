use crate::Value;
use query_template::{Fragment, QueryTemplate};

pub(crate) trait QueryWriter {
    fn write_string_chunk(&mut self, value: String);
    fn write_parameter(&mut self);
    fn write_parameter_tuple(&mut self);
}

impl QueryWriter for QueryTemplate<Value<'_>> {
    fn write_string_chunk(&mut self, value: String) {
        match self.fragments.last_mut() {
            Some(Fragment::StringChunk(chunk)) => {
                chunk.push_str(value.as_str());
            }
            _ => {
                self.fragments.push(Fragment::StringChunk(value));
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

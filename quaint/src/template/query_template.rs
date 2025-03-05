use crate::ast::Value;
use serde::Serialize;
use std::fmt::Debug;

#[derive(Debug)]
pub struct QueryTemplate<'a> {
    pub fragments: Vec<Fragment>,
    pub parameters: Vec<Value<'a>>,
    pub placeholder_format: PlaceholderFormat,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum Fragment {
    StringChunk(String),
    Parameter,
    ParameterTuple,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaceholderFormat {
    pub prefix: &'static str,
    pub has_numbering: bool,
}

impl PlaceholderFormat {
    pub fn write(&self, sql: &mut String, placeholder_number: &mut i32) {
        sql.push_str(self.prefix);
        if self.has_numbering {
            sql.push_str(placeholder_number.to_string().as_str());
            *placeholder_number += 1;
        }
    }
}

const BEGIN_REPEAT: &str = "/* prisma-comma-repeatable-start */";
const END_REPEAT: &str = "/* prisma-comma-repeatable-end */";

impl QueryTemplate<'_> {
    pub fn new(placeholder: PlaceholderFormat) -> Self {
        QueryTemplate {
            fragments: Vec::with_capacity(64),
            parameters: Vec::with_capacity(64),
            placeholder_format: placeholder,
        }
    }

    /// Used only for testing and for compatibility with the old Query Engine code
    pub fn to_sql(&self) -> String {
        let mut sql = String::with_capacity(4096);
        let mut placeholder_number = 1;
        for fragment in &self.fragments {
            match fragment {
                Fragment::StringChunk(chunk) => sql.push_str(chunk),
                Fragment::Parameter => {
                    self.placeholder_format.write(&mut sql, &mut placeholder_number);
                }

                // Code compatibility for parameter tuples (repeatable parameters)
                Fragment::ParameterTuple => {
                    sql.push_str(BEGIN_REPEAT);
                    self.placeholder_format.write(&mut sql, &mut placeholder_number);
                    sql.push_str(END_REPEAT);
                }
            };
        }
        sql
    }
}

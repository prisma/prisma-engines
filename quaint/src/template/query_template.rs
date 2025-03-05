use std::fmt::Debug;
use crate::ast::Value;

#[derive(Debug)]
pub struct QueryTemplate<'a> {
    pub fragments: Vec<Fragment>,
    pub parameters: Vec<Value<'a>>,
    pub placeholder: Placeholder,
}

#[derive(Debug)]
pub enum Fragment {
    StringChunk(String),
    Parameter,
    ParameterTuple,
}

#[derive(Debug)]
pub struct Placeholder {
    pub prefix: &'static str,
    pub has_numbering: bool,
}

impl Placeholder {
    pub fn write(&self, sql: &mut String, placeholder_number: &mut i32) {
        sql.push_str(&self.prefix);
        if self.has_numbering {
            sql.push_str(placeholder_number.to_string().as_str());
            *placeholder_number += 1;
        }
    }
}

const BEGIN_REPEAT: &'static str = "/* prisma-comma-repeatable-start */";
const END_REPEAT: &'static str = "/* prisma-comma-repeatable-end */";

impl<'a> QueryTemplate<'a> {
    pub fn new(placeholder: Placeholder) -> Self {
        QueryTemplate {
            fragments: Vec::with_capacity(64),
            parameters: Vec::with_capacity(64),
            placeholder,
        }
    }

    /// Used only for testing and for compatibility with the old Query Engine code
    pub fn to_sql(&self) -> String {
        let mut sql = String::with_capacity(4096);
        let mut placeholder_number = 1;
        for fragment in &self.fragments {
            match fragment {
                Fragment::StringChunk(chunk) => { sql.push_str(&chunk) }
                Fragment::Parameter => {
                    self.placeholder.write(&mut sql, &mut placeholder_number);
                }

                // Code compatibility for parameter tuples (repeatable parameters)
                Fragment::ParameterTuple => {
                    sql.push_str(BEGIN_REPEAT);
                    self.placeholder.write(&mut sql, &mut placeholder_number);
                    sql.push_str(END_REPEAT);
                }
            };
        }
        sql
    }
}

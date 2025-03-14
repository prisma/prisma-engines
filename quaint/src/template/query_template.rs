use crate::ast::Value;
use serde::Serialize;
use std::fmt;
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
            fragments: Vec::new(),
            parameters: Vec::new(),
            placeholder_format: placeholder,
        }
    }
}

/// Used only for testing and for compatibility with the old Query Engine code
impl fmt::Display for QueryTemplate<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn fmt_param(f: &mut fmt::Formatter<'_>, format: &PlaceholderFormat, number: &mut i32) -> fmt::Result {
            if format.has_numbering {
                write!(f, "{}{}", format.prefix, number)?;
                *number += 1;
                Ok(())
            } else {
                write!(f, "{}", format.prefix)
            }
        }

        let mut placeholder_number = 1;
        for fragment in &self.fragments {
            match fragment {
                Fragment::StringChunk(chunk) => write!(f, "{chunk}"),
                Fragment::Parameter => fmt_param(f, &self.placeholder_format, &mut placeholder_number),
                Fragment::ParameterTuple => {
                    write!(f, "{BEGIN_REPEAT}")?;
                    fmt_param(f, &self.placeholder_format, &mut placeholder_number)?;
                    write!(f, "{END_REPEAT}")
                }
            }?
        }

        Ok(())
    }
}

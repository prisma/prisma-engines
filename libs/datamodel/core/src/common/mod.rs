pub mod argument;
pub mod functions;
mod interpolation;
pub mod names;
pub mod value;

mod fromstr;
mod string_helper;

pub use fromstr::FromStrAndSpan;
pub use string_helper::WritableString;

use crate::ast;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Prisma's builtin base types.
#[derive(Debug, Copy, PartialEq, Clone, Serialize, Deserialize)]
pub enum ScalarType {
    Int,
    Float,
    Decimal,
    Boolean,
    String,
    DateTime,
}

impl ScalarType {
    pub fn from_str_and_span(s: &str, span: ast::Span) -> Result<Self, String> {
        match s {
            "Int" => Ok(ScalarType::Int),
            "Float" => Ok(ScalarType::Float),
            "Decimal" => Ok(ScalarType::Decimal),
            "Boolean" => Ok(ScalarType::Boolean),
            "String" => Ok(ScalarType::String),
            "DateTime" => Ok(ScalarType::DateTime),
            _ => Err(format!("type {} is not a know scalar type.", s)),
        }
    }
}

impl ToString for ScalarType {
    fn to_string(&self) -> String {
        match self {
            ScalarType::Int => String::from("Int"),
            ScalarType::Float => String::from("Float"),
            ScalarType::Decimal => String::from("Decimal"),
            ScalarType::Boolean => String::from("Boolean"),
            ScalarType::String => String::from("String"),
            ScalarType::DateTime => String::from("DateTime"),
        }
    }
}

/// Value types for Prisma's builtin base types.
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum ScalarValue {
    Int(i32),
    Float(f32),
    Decimal(f32),
    Boolean(bool),
    String(String),
    DateTime(DateTime<Utc>),
    ConstantLiteral(String),
    Expression(String, ScalarType, Vec<ScalarValue>),
}

impl ScalarValue {
    fn get_type(&self) -> ScalarType {
        match self {
            ScalarValue::Int(_) => ScalarType::Int,
            ScalarValue::Float(_) => ScalarType::Float,
            ScalarValue::Decimal(_) => ScalarType::Decimal,
            ScalarValue::Boolean(_) => ScalarType::Boolean,
            ScalarValue::String(_) => ScalarType::String,
            ScalarValue::DateTime(_) => ScalarType::DateTime,
            ScalarValue::Expression(_, t, _) => *t,
            ScalarValue::ConstantLiteral(_) => {
                panic!("Constant literal values do not map to a base type and should never surface.")
            }
        }
    }
}

impl ToString for ScalarValue {
    fn to_string(&self) -> String {
        match self {
            ScalarValue::Int(val) => val.to_string(),
            ScalarValue::Float(val) => val.to_string(),
            ScalarValue::Decimal(val) => val.to_string(),
            ScalarValue::Boolean(val) => val.to_string(),
            ScalarValue::String(val) => val.to_string(),
            ScalarValue::DateTime(val) => val.to_string(),
            ScalarValue::ConstantLiteral(val) => val.to_string(),
            ScalarValue::Expression(_, t, _) => format!("Function<{}>", t.to_string()),
        }
    }
}

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
pub enum PrismaValue {
    Int(i32),
    Float(f32),
    Decimal(f32),
    Boolean(bool),
    String(String),
    DateTime(DateTime<Utc>),
    ConstantLiteral(String),
    Expression(String, ScalarType, Vec<PrismaValue>),
}

impl PrismaValue {
    fn get_type(&self) -> ScalarType {
        match self {
            PrismaValue::Int(_) => ScalarType::Int,
            PrismaValue::Float(_) => ScalarType::Float,
            PrismaValue::Decimal(_) => ScalarType::Decimal,
            PrismaValue::Boolean(_) => ScalarType::Boolean,
            PrismaValue::String(_) => ScalarType::String,
            PrismaValue::DateTime(_) => ScalarType::DateTime,
            PrismaValue::Expression(_, t, _) => *t,
            PrismaValue::ConstantLiteral(_) => {
                panic!("Constant literal values do not map to a base type and should never surface.")
            }
        }
    }
}

impl ToString for PrismaValue {
    fn to_string(&self) -> String {
        match self {
            PrismaValue::Int(val) => val.to_string(),
            PrismaValue::Float(val) => val.to_string(),
            PrismaValue::Decimal(val) => val.to_string(),
            PrismaValue::Boolean(val) => val.to_string(),
            PrismaValue::String(val) => val.to_string(),
            PrismaValue::DateTime(val) => val.to_string(),
            PrismaValue::ConstantLiteral(val) => val.to_string(),
            PrismaValue::Expression(_, t, _) => format!("Function<{}>", t.to_string()),
        }
    }
}

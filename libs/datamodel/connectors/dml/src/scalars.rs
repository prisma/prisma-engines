use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Prisma's builtin scalar types.
#[derive(Debug, Copy, PartialEq, Clone, Serialize, Deserialize, Eq, Hash)]
pub enum ScalarType {
    Int,
    BigInt,
    Float,
    Boolean,
    String,
    DateTime,
    Json,
    Bytes,
    Decimal,
}

impl FromStr for ScalarType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Int" => Ok(ScalarType::Int),
            "BigInt" => Ok(ScalarType::BigInt),
            "Float" => Ok(ScalarType::Float),
            "Boolean" => Ok(ScalarType::Boolean),
            "String" => Ok(ScalarType::String),
            "DateTime" => Ok(ScalarType::DateTime),
            "Json" => Ok(ScalarType::Json),
            "Bytes" => Ok(ScalarType::Bytes),
            "Decimal" => Ok(ScalarType::Decimal),
            _ => Err(format!("type {} is not a known scalar type.", s)),
        }
    }
}

impl ToString for ScalarType {
    fn to_string(&self) -> String {
        match self {
            ScalarType::Int => String::from("Int"),
            ScalarType::BigInt => String::from("BigInt"),
            ScalarType::Float => String::from("Float"),
            ScalarType::Boolean => String::from("Boolean"),
            ScalarType::String => String::from("String"),
            ScalarType::DateTime => String::from("DateTime"),
            ScalarType::Json => String::from("Json"),
            ScalarType::Bytes => String::from("Bytes"),
            ScalarType::Decimal => String::from("Decimal"),
        }
    }
}

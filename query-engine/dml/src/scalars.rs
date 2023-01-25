use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

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

impl ScalarType {
    pub fn is_boolean(&self) -> bool {
        matches!(self, ScalarType::Boolean)
    }

    pub fn is_datetime(&self) -> bool {
        matches!(self, ScalarType::DateTime)
    }

    pub fn is_float(&self) -> bool {
        matches!(self, ScalarType::Float)
    }

    pub fn is_json(&self) -> bool {
        matches!(self, ScalarType::Json)
    }

    pub fn is_string(&self) -> bool {
        matches!(self, ScalarType::String)
    }
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

impl AsRef<str> for ScalarType {
    fn as_ref(&self) -> &str {
        match self {
            ScalarType::Int => "Int",
            ScalarType::BigInt => "BigInt",
            ScalarType::Float => "Float",
            ScalarType::Boolean => "Boolean",
            ScalarType::String => "String",
            ScalarType::DateTime => "DateTime",
            ScalarType::Json => "Json",
            ScalarType::Bytes => "Bytes",
            ScalarType::Decimal => "Decimal",
        }
    }
}

impl fmt::Display for ScalarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_ref().fmt(f)
    }
}

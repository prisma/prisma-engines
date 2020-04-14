use serde::{Deserialize, Serialize};

/// Prisma's builtin scalar types.
#[derive(Debug, Copy, PartialEq, Clone, Serialize, Deserialize, Eq, Hash)]
pub enum ScalarType {
    Int,
    Float,
    Boolean,
    String,
    DateTime,
}

impl ScalarType {
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "Int" => Ok(ScalarType::Int),
            "Float" => Ok(ScalarType::Float),
            "Boolean" => Ok(ScalarType::Boolean),
            "String" => Ok(ScalarType::String),
            "DateTime" => Ok(ScalarType::DateTime),
            _ => Err(format!("type {} is not a known scalar type.", s)),
        }
    }
}

impl ToString for ScalarType {
    fn to_string(&self) -> String {
        match self {
            ScalarType::Int => String::from("Int"),
            ScalarType::Float => String::from("Float"),
            ScalarType::Boolean => String::from("Boolean"),
            ScalarType::String => String::from("String"),
            ScalarType::DateTime => String::from("DateTime"),
        }
    }
}

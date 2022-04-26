use std::fmt;

use serde::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostgresType {
    SmallInt,
    Integer,
    BigInt,
    Decimal(Option<(u32, u32)>),
    Money,
    Inet,
    Oid,
    Citext,
    Real,
    DoublePrecision,
    VarChar(Option<u32>),
    Char(Option<u32>),
    Text,
    ByteA,
    Timestamp(Option<u32>),
    Timestamptz(Option<u32>),
    Date,
    Time(Option<u32>),
    Timetz(Option<u32>),
    Boolean,
    Bit(Option<u32>),
    VarBit(Option<u32>),
    Uuid,
    Xml,
    Json,
    JsonB,
}

impl fmt::Display for PostgresType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PostgresType::SmallInt => f.write_str("SmallInt"),
            PostgresType::Integer => f.write_str("Integer"),
            PostgresType::BigInt => f.write_str("BigInt"),
            PostgresType::Decimal(_) => f.write_str("Decimal"),
            PostgresType::Money => f.write_str("Money"),
            PostgresType::Inet => f.write_str("Inet"),
            PostgresType::Oid => f.write_str("Oid"),
            PostgresType::Citext => f.write_str("Citext"),
            PostgresType::Real => f.write_str("Real"),
            PostgresType::DoublePrecision => f.write_str("DoublePrecision"),
            PostgresType::VarChar(_) => f.write_str("VarChar"),
            PostgresType::Char(_) => f.write_str("Char"),
            PostgresType::Text => f.write_str("Text"),
            PostgresType::ByteA => f.write_str("ByteA"),
            PostgresType::Timestamp(_) => f.write_str("Timestamp"),
            PostgresType::Timestamptz(_) => f.write_str("Timestamptz"),
            PostgresType::Date => f.write_str("Date"),
            PostgresType::Time(_) => f.write_str("Time"),
            PostgresType::Timetz(_) => f.write_str("Timetz"),
            PostgresType::Boolean => f.write_str("Boolean"),
            PostgresType::Bit(_) => f.write_str("Bit"),
            PostgresType::VarBit(_) => f.write_str("VarBit"),
            PostgresType::Uuid => f.write_str("Uuid"),
            PostgresType::Xml => f.write_str("Xml"),
            PostgresType::Json => f.write_str("Json"),
            PostgresType::JsonB => f.write_str("JsonB"),
        }
    }
}

impl super::NativeType for PostgresType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

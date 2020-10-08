use serde::*;
use serde_json::Value;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Copy)]
pub enum DataLength {
    Limited(u16),
    Max,
}

impl fmt::Display for DataLength {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limited(length) => write!(f, "{}", length),
            Self::Max => write!(f, "max"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MssqlType {
    TinyInt,
    SmallInt,
    Int,
    BigInt,
    Decimal(u8, u8),
    Numeric(u8, u8),
    Money,
    SmallMoney,
    Bit,
    Float(u8),
    Real,
    Date,
    Time,
    Datetime,
    Datetime2,
    DatetimeOffset,
    SmallDatetime,
    Char(DataLength),
    VarChar(DataLength),
    Text,
    NVarChar(DataLength),
    NText,
    Binary(DataLength),
    VarBinary(DataLength),
    Image,
    XML,
}

impl super::NativeType for MssqlType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self).unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

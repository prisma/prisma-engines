#![deny(missing_docs)]

use serde::*;
use serde_json::Value;

/// The MySQL native type enum.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum MySqlType {
    Int,
    UnsignedInt,
    SmallInt,
    UnsignedSmallInt,
    TinyInt,
    UnsignedTinyInt,
    MediumInt,
    UnsignedMediumInt,
    BigInt,
    Decimal(Option<(u32, u32)>),
    UnsignedBigInt,
    Float,
    Double,
    Bit(u32),
    Char(u32),
    VarChar(u32),
    Binary(u32),
    VarBinary(u32),
    TinyBlob,
    Blob,
    MediumBlob,
    LongBlob,
    TinyText,
    Text,
    MediumText,
    LongText,
    Date,
    Time(Option<u32>),
    DateTime(Option<u32>),
    Timestamp(Option<u32>),
    Year,
    Json,
}

impl MySqlType {
    /// The user-defined precision for timestamp columns, where applicable.
    pub fn timestamp_precision(&self) -> Option<u32> {
        match self {
            MySqlType::Time(n) | MySqlType::DateTime(n) | MySqlType::Timestamp(n) => *n,
            _ => None,
        }
    }
}

impl super::NativeType for MySqlType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

use serde::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    Decimal(u32, u32),
    Numeric(u32, u32),
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
    Time(u32),
    DateTime(u32),
    Timestamp(u32),
    Year,
    JSON,
    Set,
}

impl super::NativeType for MySqlType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self).expect(&format!("Serializing the native type to json failed: {:?}", &self))
    }
}

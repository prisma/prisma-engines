use serde::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostgresType {
    SmallInt,
    Integer,
    BigInt,
    Decimal(Option<(u32, u32)>),
    Numeric(Option<(u32, u32)>),
    Real,
    DoublePrecision,
    SmallSerial,
    Serial,
    BigSerial,
    VarChar(Option<u32>),
    Char(Option<u32>),
    Text,
    ByteA,
    Timestamp(Option<u32>),
    Date,
    Time(Option<u32>),
    Boolean,
    Bit(Option<u32>),
    VarBit(Option<u32>),
    UUID,
    Xml,
    JSON,
    JSONB,
}

impl super::NativeType for PostgresType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

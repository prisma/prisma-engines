use serde::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostgresType {
    SmallInt,
    Integer,
    BigInt,
    Numeric(u8, u8),
    Real,
    DoublePrecision,
    SmallSerial,
    Serial,
    BigSerial,
    VarChar(u32),
    Char(u32),
    Text,
    ByteA,
    Timestamp(u8),
    TimestampWithTimeZone(u8),
    Date,
    Time(u8),
    TimeWithTimeZone(u8),
    Interval(u8),
    Boolean,
    Bit(u32),
    VarBit(u32),
    UUID,
    XML,
    JSON,
    JSONB,
}

impl super::NativeType for PostgresType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self).expect(&format!("Serializing the native type to json failed: {:?}", &self))
    }
}

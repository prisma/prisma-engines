use serde::*;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PostgresType {
    SmallInt,
    Integer,
    BigInt,
    Decimal(u32, u32),
    Numeric(u32, u32),
    Real,
    DoublePrecision,
    SmallSerial,
    Serial,
    BigSerial,
    VarChar(u32),
    Char(u32),
    Text,
    ByteA,
    Timestamp(u32),
    TimestampWithTimeZone(u32),
    Date,
    Time(u32),
    TimeWithTimeZone(u32),
    Interval(u32),
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

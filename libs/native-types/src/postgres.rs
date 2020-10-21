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
    Timestamp(Option<u32>),
    TimestampWithTimeZone(Option<u32>),
    Date,
    Time(Option<u32>),
    TimeWithTimeZone(Option<u32>),
    Interval(Option<u32>),
    Boolean,
    Bit(u32),
    VarBit(u32),
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

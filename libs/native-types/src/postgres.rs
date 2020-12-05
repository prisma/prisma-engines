use crate::NativeType;

#[derive(Debug, Clone, PartialEq)]
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
    Timestamptz(Option<u32>),
    Date,
    Time(Option<u32>),
    Timetz(Option<u32>),
    Boolean,
    Bit(Option<u32>),
    VarBit(Option<u32>),
    UUID,
    Xml,
    JSON,
    JSONB,
}

impl PostgresType {
    pub fn as_native_type(self) -> NativeType {
        NativeType::Postgres(self)
    }
}

use serde::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MySqlType {
    Int,
    SmallInt,
    TinyInt,
    MediumInt,
    BigInt,
    Decimal(u8, u8),
    Numeric(u8, u8),
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
    Time(Option<u32>), // todo carmen how to handle optional argument in sql connector?
    DateTime(Option<u32>),
    Timestamp(Option<u32>),
    Year,
    JSON,
}

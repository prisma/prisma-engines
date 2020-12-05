use crate::NativeType;

#[derive(Debug, Clone, PartialEq)]
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
    Numeric(Option<(u32, u32)>),
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
    JSON,
}

impl MySqlType {
    pub fn as_native_type(self) -> NativeType {
        NativeType::MySQL(self)
    }
}

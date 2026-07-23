use std::borrow::Cow;

#[derive(Debug, Clone, PartialEq)]
pub enum PostgresType {
    Known(KnownPostgresType),
    Unknown(String, Vec<String>),
}

impl PostgresType {
    pub fn to_parts(&self) -> (&str, Cow<'_, [String]>) {
        match self {
            Self::Known(known) => known.to_parts(),
            Self::Unknown(name, args) => (name.as_str(), Cow::Borrowed(args)),
        }
    }

    pub fn as_known(&self) -> Option<&KnownPostgresType> {
        match self {
            Self::Known(known) => Some(known),
            Self::Unknown(_, _) => None,
        }
    }
}

crate::native_type_definition! {
    KnownPostgresType;
    SmallInt -> Int,
    Integer -> Int,
    BigInt -> BigInt,
    Decimal(Option<(u32, u32)>) -> Decimal,
    Money -> Decimal,
    Inet -> String,
    Oid -> Int,
    Citext -> String,
    Real -> Float,
    DoublePrecision -> Float,
    VarChar(Option<u32>) -> String,
    Char(Option<u32>) -> String,
    Text -> String,
    ByteA -> Bytes,
    Timestamp(Option<u32>) -> DateTime,
    Timestamptz(Option<u32>) -> DateTime,
    Date -> DateTime,
    Time(Option<u32>) -> DateTime,
    Timetz(Option<u32>) -> DateTime,
    Boolean -> Boolean,
    Bit(Option<u32>) -> String,
    VarBit(Option<u32>) -> String,
    Uuid -> String,
    Xml -> String,
    Json -> Json,
    JsonB -> Json,
}

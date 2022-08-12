use serde::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CockroachType {
    Bit(Option<u32>),
    Bool,
    Bytes,
    Char(Option<u32>),
    Date,
    Decimal(Option<(u32, u32)>),
    Float4,
    Float8,
    Inet,
    Int2,
    Int4,
    Int8,
    JsonB,
    Oid,
    CatalogSingleChar,
    String(Option<u32>),
    Time(Option<u32>),
    Timestamp(Option<u32>),
    Timestamptz(Option<u32>),
    Timetz(Option<u32>),
    Uuid,
    VarBit(Option<u32>),
}

impl super::NativeType for CockroachType {
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

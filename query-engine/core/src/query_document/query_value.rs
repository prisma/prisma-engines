use prisma_value::{stringify_date, PrismaValue};
use rust_decimal::Decimal;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum QueryValue {
    Int(i64),
    Float(Decimal),
    String(String),
    Boolean(bool),
    Null,
    Enum(String),
    List(Vec<QueryValue>),
    Object(BTreeMap<String, QueryValue>),
}

impl QueryValue {
    pub fn into_object(self) -> Option<BTreeMap<String, QueryValue>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }
}

impl From<PrismaValue> for QueryValue {
    fn from(pv: PrismaValue) -> Self {
        match pv {
            PrismaValue::String(s) => Self::String(s),
            PrismaValue::Float(f) => Self::Float(f),
            PrismaValue::Boolean(b) => Self::Boolean(b),
            PrismaValue::DateTime(dt) => Self::String(stringify_date(&dt)),
            PrismaValue::Enum(s) => Self::Enum(s),
            PrismaValue::List(l) => Self::List(l.into_iter().map(QueryValue::from).collect()),
            PrismaValue::Int(i) => Self::Int(i),
            PrismaValue::Null => Self::Null,
            PrismaValue::Uuid(u) => Self::String(u.to_hyphenated().to_string()),
            PrismaValue::Json(s) => Self::String(s),
        }
    }
}

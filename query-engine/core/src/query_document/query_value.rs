use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use prisma_value::{stringify_date, PrismaValue};
use std::hash::Hash;

#[derive(Debug, Clone, Eq)]
pub enum QueryValue {
    Int(i64),
    Float(BigDecimal),
    String(String),
    Boolean(bool),
    Null,
    Enum(String),
    List(Vec<QueryValue>),
    Object(IndexMap<String, QueryValue>),
}

impl PartialEq for QueryValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (QueryValue::Int(n1), QueryValue::Int(n2)) => n1 == n2,
            (QueryValue::Float(n1), QueryValue::Float(n2)) => n1 == n2,
            (QueryValue::String(s1), QueryValue::String(s2)) => s1 == s2,
            (QueryValue::Boolean(b1), QueryValue::Boolean(b2)) => b1 == b2,
            (QueryValue::Null, QueryValue::Null) => true,
            (QueryValue::Enum(kind1), QueryValue::Enum(kind2)) => kind1 == kind2,
            (QueryValue::List(list1), QueryValue::List(list2)) => list1 == list2,
            (QueryValue::Object(t1), QueryValue::Object(t2)) => t1 == t2,
            _ => false,
        }
    }
}

impl Hash for QueryValue {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Self::Int(i) => i.hash(state),
            Self::Float(f) => f.hash(state),
            Self::String(s) => s.hash(state),
            Self::Boolean(b) => b.hash(state),
            Self::Null => (),
            Self::Enum(s) => s.hash(state),
            Self::List(l) => l.hash(state),
            Self::Object(map) => {
                let converted: std::collections::BTreeMap<&String, &QueryValue> = map.into_iter().collect();
                converted.hash(state);
            }
        }
    }
}

impl QueryValue {
    pub fn into_object(self) -> Option<IndexMap<String, QueryValue>> {
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
            PrismaValue::Uuid(u) => Self::String(u.hyphenated().to_string()),
            PrismaValue::Json(s) => Self::String(s),
            PrismaValue::Xml(s) => Self::String(s),
            PrismaValue::Bytes(b) => Self::String(prisma_value::encode_bytes(&b)),
            PrismaValue::BigInt(i) => Self::Int(i),
            PrismaValue::Object(pairs) => Self::Object(pairs.into_iter().map(|(k, v)| (k, v.into())).collect()),
        }
    }
}

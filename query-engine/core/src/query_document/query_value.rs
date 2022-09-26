use bigdecimal::BigDecimal;
use indexmap::IndexMap;
use prisma_value::{stringify_date, PrismaValue};

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
    DateTime(chrono::DateTime<chrono::FixedOffset>),
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
            (QueryValue::DateTime(t1), QueryValue::DateTime(t2)) => t1 == t2,
            (QueryValue::String(t1), QueryValue::DateTime(t2)) | (QueryValue::DateTime(t2), QueryValue::String(t1)) => {
                chrono::DateTime::parse_from_rfc3339(t1)
                    .map(|t1| &t1 == t2)
                    .unwrap_or_else(|_| t1 == stringify_date(t2).as_str())
            }
            _ => false,
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
            PrismaValue::DateTime(dt) => Self::DateTime(dt),
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

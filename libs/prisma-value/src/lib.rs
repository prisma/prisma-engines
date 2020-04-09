mod error;
#[cfg(feature = "sql-ext")]
pub mod sql_ext;

use chrono::prelude::*;
use rust_decimal::{prelude::ToPrimitive, Decimal};
use serde::{ser::Serializer, Deserialize, Serialize};
use std::{convert::TryFrom, fmt, str::FromStr};
use uuid::Uuid;

pub use error::ConversionFailure;
pub type PrismaValueResult<T> = std::result::Result<T, ConversionFailure>;
pub type PrismaListValue = Vec<PrismaValue>;

use rust_decimal::prelude::FromPrimitive;
#[cfg(feature = "sql-ext")]
pub use sql_ext::*;

#[derive(Debug, PartialEq, Clone, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(untagged)]
pub enum PrismaValue {
    String(String),
    Boolean(bool),
    Enum(String),
    Int(i64),
    Null,
    Uuid(Uuid),
    List(PrismaListValue),

    #[serde(serialize_with = "serialize_date")]
    DateTime(DateTime<Utc>),

    #[serde(serialize_with = "serialize_decimal")]
    Float(Decimal),
}

pub fn stringify_date(date: &DateTime<Utc>) -> String {
    format!("{}", date.format("%Y-%m-%dT%H:%M:%S%.3fZ"))
}

impl TryFrom<serde_json::Value> for PrismaValue {
    type Error = crate::error::ConversionFailure;

    fn try_from(v: serde_json::Value) -> PrismaValueResult<Self> {
        match v {
            serde_json::Value::String(s) => Ok(PrismaValue::String(s)),
            serde_json::Value::Array(v) => {
                let vals: PrismaValueResult<Vec<PrismaValue>> = v.into_iter().map(PrismaValue::try_from).collect();
                Ok(PrismaValue::List(vals?))
            }
            serde_json::Value::Null => Ok(PrismaValue::Null),
            serde_json::Value::Bool(b) => Ok(PrismaValue::Boolean(b)),
            serde_json::Value::Number(num) => {
                if num.is_i64() {
                    Ok(PrismaValue::Int(num.as_i64().unwrap()))
                } else {
                    let fl = num.as_f64().unwrap();
                    // Decimal::from_f64 is buggy. Issue: https://github.com/paupino/rust-decimal/issues/228
                    let dec = Decimal::from_str(&fl.to_string()).unwrap();

                    Ok(PrismaValue::Float(dec))
                }
            }
            serde_json::Value::Object(_) => Err(ConversionFailure::new("nested JSON object", "PrismaValue")),
        }
    }
}

fn serialize_date<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    format!("{}", stringify_date(date)).serialize(serializer)
}

fn serialize_decimal<S>(decimal: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    decimal.to_f64().expect("Decimal is not a f64.").serialize(serializer)
}

impl PrismaValue {
    pub fn is_null(&self) -> bool {
        match self {
            PrismaValue::Null => true,
            _ => false,
        }
    }

    pub fn into_string(self) -> Option<String> {
        match self {
            PrismaValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_list(self) -> Option<PrismaListValue> {
        match self {
            PrismaValue::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn new_float(float: f64) -> PrismaValue {
        PrismaValue::Float(Decimal::from_f64(float).unwrap())
    }

    pub fn new_datetime(datetime: &str) -> PrismaValue {
        PrismaValue::DateTime(DateTime::<Utc>::from_str(datetime).unwrap())
    }
}

impl fmt::Display for PrismaValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrismaValue::String(x) => x.fmt(f),
            PrismaValue::Float(x) => x.fmt(f),
            PrismaValue::Boolean(x) => x.fmt(f),
            PrismaValue::DateTime(x) => x.fmt(f),
            PrismaValue::Enum(x) => x.fmt(f),
            PrismaValue::Int(x) => x.fmt(f),
            PrismaValue::Null => "null".fmt(f),
            PrismaValue::Uuid(x) => x.fmt(f),
            PrismaValue::List(x) => {
                let as_string = format!("{:?}", x);
                as_string.fmt(f)
            }
        }
    }
}

impl From<&str> for PrismaValue {
    fn from(s: &str) -> Self {
        PrismaValue::from(s.to_string())
    }
}

impl From<String> for PrismaValue {
    fn from(s: String) -> Self {
        PrismaValue::String(s)
    }
}

impl TryFrom<f64> for PrismaValue {
    type Error = ConversionFailure;

    fn try_from(f: f64) -> PrismaValueResult<PrismaValue> {
        // Decimal::from_f64 is buggy. Issue: https://github.com/paupino/rust-decimal/issues/228
        Decimal::from_str(&f.to_string())
            .ok()
            .map(|d| PrismaValue::Float(d))
            .ok_or(ConversionFailure::new("f64", "Decimal"))
    }
}

impl From<bool> for PrismaValue {
    fn from(b: bool) -> Self {
        PrismaValue::Boolean(b)
    }
}

impl From<i32> for PrismaValue {
    fn from(i: i32) -> Self {
        PrismaValue::Int(i64::from(i))
    }
}

impl From<i64> for PrismaValue {
    fn from(i: i64) -> Self {
        PrismaValue::Int(i)
    }
}

impl From<usize> for PrismaValue {
    fn from(u: usize) -> Self {
        PrismaValue::Int(u as i64)
    }
}

impl From<Uuid> for PrismaValue {
    fn from(s: Uuid) -> Self {
        PrismaValue::Uuid(s)
    }
}

impl From<PrismaListValue> for PrismaValue {
    fn from(s: PrismaListValue) -> Self {
        PrismaValue::List(s)
    }
}

impl TryFrom<PrismaValue> for i64 {
    type Error = ConversionFailure;

    fn try_from(value: PrismaValue) -> PrismaValueResult<i64> {
        match value {
            PrismaValue::Int(i) => Ok(i),
            _ => Err(ConversionFailure::new("PrismaValue", "i64")),
        }
    }
}

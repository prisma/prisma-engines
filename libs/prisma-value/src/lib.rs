pub mod arithmetic;

mod error;

use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::prelude::*;
use serde::de::Unexpected;
use serde::ser::SerializeMap;
use serde::{ser::Serializer, Deserialize, Deserializer, Serialize};
use std::{convert::TryFrom, fmt, str::FromStr};
use uuid::Uuid;

pub use error::ConversionFailure;
pub type PrismaValueResult<T> = std::result::Result<T, ConversionFailure>;
pub type PrismaListValue = Vec<PrismaValue>;

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(untagged)]
pub enum PrismaValue {
    String(String),
    Boolean(bool),
    Enum(String),
    Int(i64),
    Uuid(Uuid),
    List(PrismaListValue),
    Json(String),
    Xml(String),

    /// A collections of key-value pairs constituting an object.
    #[serde(serialize_with = "serialize_object")]
    Object(Vec<(String, PrismaValue)>),

    #[serde(serialize_with = "serialize_null")]
    Null,

    #[serde(serialize_with = "serialize_date")]
    DateTime(DateTime<FixedOffset>),

    #[serde(serialize_with = "serialize_decimal", deserialize_with = "deserialize_decimal")]
    Float(BigDecimal),

    #[serde(serialize_with = "serialize_bigint")]
    BigInt(i64),

    #[serde(serialize_with = "serialize_bytes")]
    Bytes(Vec<u8>),
}

/// Stringify a date to the following format
/// 1999-05-01T00:00:00.000Z
pub fn stringify_datetime(datetime: &DateTime<FixedOffset>) -> String {
    // Warning: Be careful if you plan on changing the code below
    // The findUnique batch optimization expects date inputs to have exactly the same format as date outputs
    // This works today because clients always send date inputs in the same format as the serialized format below
    // Updating this without transforming date inputs to the same format WILL break the findUnique batch optimization
    datetime.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Parses an RFC 3339 and ISO 8601 date and time string such as 1996-12-19T16:39:57-08:00,
/// then returns a new DateTime with a parsed FixedOffset.
pub fn parse_datetime(datetime: &str) -> chrono::ParseResult<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(datetime)
}

pub fn encode_bytes(bytes: &[u8]) -> String {
    base64::encode(bytes)
}

pub fn decode_bytes(s: &str) -> PrismaValueResult<Vec<u8>> {
    base64::decode(s).map_err(|_| ConversionFailure::new("base64 encoded bytes", "PrismaValue::Bytes"))
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
                    let dec = BigDecimal::from_f64(fl).unwrap().normalized();

                    Ok(PrismaValue::Float(dec))
                }
            }
            serde_json::Value::Object(obj) => match obj.get("prisma__type").as_ref().and_then(|s| s.as_str()) {
                Some("date") => {
                    let value = obj
                        .get("prisma__value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ConversionFailure::new("JSON date object", "PrismaValue"))?;

                    let date = DateTime::parse_from_rfc3339(value)
                        .map_err(|_| ConversionFailure::new("JSON date object", "PrismaValue"))?;

                    Ok(PrismaValue::DateTime(date))
                }
                Some("bigint") => {
                    let value = obj
                        .get("prisma__value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ConversionFailure::new("JSON bigint value", "PrismaValue"))?;

                    i64::from_str(value)
                        .map(PrismaValue::BigInt)
                        .map_err(|_| ConversionFailure::new("JSON bigint value", "PrismaValue"))
                }
                Some("decimal") => {
                    let value = obj
                        .get("prisma__value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ConversionFailure::new("JSON decimal value", "PrismaValue"))?;

                    BigDecimal::from_str(value)
                        .map(PrismaValue::Float)
                        .map_err(|_| ConversionFailure::new("JSON decimal value", "PrismaValue"))
                }
                Some("bytes") => {
                    let value = obj
                        .get("prisma__value")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ConversionFailure::new("JSON bytes value", "PrismaValue"))?;

                    decode_bytes(value).map(PrismaValue::Bytes)
                }

                _ => Ok(PrismaValue::Json(serde_json::to_string(&obj).unwrap())),
            },
        }
    }
}

fn serialize_date<S>(date: &DateTime<FixedOffset>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    stringify_datetime(date).serialize(serializer)
}

fn serialize_bytes<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    encode_bytes(bytes).serialize(serializer)
}

fn serialize_null<S>(serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    Option::<u8>::None.serialize(serializer)
}

fn serialize_bigint<S>(int: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    int.to_string().serialize(serializer)
}

fn serialize_decimal<S>(decimal: &BigDecimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    decimal.to_string().parse::<f64>().unwrap().serialize(serializer)
}

fn deserialize_decimal<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_f64(BigDecimalVisitor)
}

fn serialize_object<S>(obj: &Vec<(String, PrismaValue)>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(obj.len()))?;

    for (k, v) in obj {
        map.serialize_entry(k, v)?;
    }

    map.end()
}

struct BigDecimalVisitor;

impl<'de> serde::de::Visitor<'de> for BigDecimalVisitor {
    type Value = BigDecimal;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a BigDecimal type representing a fixed-point number")
    }

    fn visit_i64<E>(self, value: i64) -> Result<BigDecimal, E>
    where
        E: serde::de::Error,
    {
        match BigDecimal::from_i64(value) {
            Some(s) => Ok(s),
            None => Err(E::invalid_value(Unexpected::Signed(value), &self)),
        }
    }

    fn visit_u64<E>(self, value: u64) -> Result<BigDecimal, E>
    where
        E: serde::de::Error,
    {
        match BigDecimal::from_u64(value) {
            Some(s) => Ok(s),
            None => Err(E::invalid_value(Unexpected::Unsigned(value), &self)),
        }
    }

    fn visit_f64<E>(self, value: f64) -> Result<BigDecimal, E>
    where
        E: serde::de::Error,
    {
        BigDecimal::from_f64(value).ok_or_else(|| E::invalid_value(Unexpected::Float(value), &self))
    }

    fn visit_str<E>(self, value: &str) -> Result<BigDecimal, E>
    where
        E: serde::de::Error,
    {
        BigDecimal::from_str(value).map_err(|_| E::invalid_value(Unexpected::Str(value), &self))
    }
}

impl PrismaValue {
    pub fn as_enum_value(&self) -> Option<&str> {
        match self {
            PrismaValue::Enum(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            PrismaValue::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            PrismaValue::Bytes(s) => Some(s),
            _ => None,
        }
    }

    /// For reexport convenience.
    pub fn decode_bytes(s: &str) -> PrismaValueResult<Vec<u8>> {
        decode_bytes(s)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, PrismaValue::Null)
    }

    pub fn into_string(self) -> Option<String> {
        match self {
            PrismaValue::String(s) => Some(s),
            PrismaValue::Enum(ev) => Some(ev),
            _ => None,
        }
    }

    pub fn into_list(self) -> Option<PrismaListValue> {
        match self {
            PrismaValue::List(l) => Some(l),
            _ => None,
        }
    }

    pub fn into_object(self) -> Option<Vec<(String, PrismaValue)>> {
        match self {
            PrismaValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn new_float(float: f64) -> PrismaValue {
        PrismaValue::Float(BigDecimal::from_f64(float).unwrap())
    }

    pub fn new_datetime(datetime: &str) -> PrismaValue {
        PrismaValue::DateTime(parse_datetime(datetime).unwrap())
    }

    pub fn as_boolean(&self) -> Option<&bool> {
        match self {
            PrismaValue::Boolean(bool) => Some(bool),
            _ => None,
        }
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
            PrismaValue::Json(x) => x.fmt(f),
            PrismaValue::Xml(x) => x.fmt(f),
            PrismaValue::BigInt(x) => x.fmt(f),
            PrismaValue::List(x) => {
                let as_string = format!("{x:?}");
                as_string.fmt(f)
            }
            PrismaValue::Bytes(b) => encode_bytes(b).fmt(f),
            PrismaValue::Object(pairs) => {
                let joined = pairs
                    .iter()
                    .map(|(key, value)| format!(r#""{key}": {value}"#))
                    .collect::<Vec<_>>()
                    .join(", ");

                write!(f, "{{ {joined} }}")
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
        BigDecimal::from_f64(f)
            .map(PrismaValue::Float)
            .ok_or_else(|| ConversionFailure::new("f64", "Decimal"))
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

impl TryFrom<PrismaValue> for String {
    type Error = ConversionFailure;

    fn try_from(pv: PrismaValue) -> PrismaValueResult<String> {
        match pv {
            PrismaValue::String(s) => Ok(s),
            _ => Err(ConversionFailure::new("PrismaValue", "String")),
        }
    }
}

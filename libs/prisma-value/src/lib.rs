pub mod arithmetic;

mod error;
mod raw_json;
mod tagged;

use base64::prelude::*;
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::prelude::*;
use serde::de::Unexpected;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, ser::Serializer};
use serde_json::json;
use std::{borrow::Cow, convert::TryFrom, fmt, str::FromStr};
use uuid::Uuid;

pub use error::ConversionFailure;
pub use raw_json::RawJson;
pub use tagged::TaggedPrismaValue;

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

    #[serde(serialize_with = "serialize_placeholder")]
    Placeholder(Placeholder),

    #[serde(serialize_with = "serialize_generator_call")]
    GeneratorCall {
        name: Cow<'static, str>,
        args: Vec<Self>,
        return_type: PrismaValueType,
    },
}

impl PrismaValue {
    pub fn r#type(&self) -> PrismaValueType {
        match self {
            PrismaValue::String(_) => PrismaValueType::String,
            PrismaValue::Boolean(_) => PrismaValueType::Boolean,
            PrismaValue::Int(_) => PrismaValueType::Int,
            PrismaValue::Uuid(_) => PrismaValueType::String,
            PrismaValue::List(_) => PrismaValueType::Array(PrismaValueType::Any.into()),
            PrismaValue::Json(_) => PrismaValueType::Json,
            PrismaValue::Object(_) => PrismaValueType::Object,
            PrismaValue::DateTime(_) => PrismaValueType::Date,
            PrismaValue::Float(_) => PrismaValueType::Float,
            PrismaValue::BigInt(_) => PrismaValueType::BigInt,
            PrismaValue::Bytes(_) => PrismaValueType::Bytes,
            PrismaValue::Placeholder(placeholder) => placeholder.r#type.clone(),
            PrismaValue::GeneratorCall { return_type, .. } => return_type.clone(),
            PrismaValue::Enum(_) => PrismaValueType::Any, // we don't know the enum type at this point
            PrismaValue::Null => PrismaValueType::Any,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(tag = "type", content = "inner")]
pub enum PrismaValueType {
    Any,
    String,
    Int,
    BigInt,
    Float,
    Boolean,
    Decimal,
    Date,
    Time,
    Array(Box<PrismaValueType>),
    Json,
    Object,
    Bytes,
    Enum(String),
}

impl std::fmt::Display for PrismaValueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrismaValueType::Any => write!(f, "Any"),
            PrismaValueType::String => write!(f, "String"),
            PrismaValueType::Int => write!(f, "Int"),
            PrismaValueType::BigInt => write!(f, "BigInt"),
            PrismaValueType::Float => write!(f, "Float"),
            PrismaValueType::Boolean => write!(f, "Boolean"),
            PrismaValueType::Decimal => write!(f, "Decimal"),
            PrismaValueType::Date => write!(f, "Date"),
            PrismaValueType::Time => write!(f, "Time"),
            PrismaValueType::Array(t) => write!(f, "Array<{t}>"),
            PrismaValueType::Json => write!(f, "Json"),
            PrismaValueType::Object => write!(f, "Object"),
            PrismaValueType::Bytes => write!(f, "Bytes"),
            PrismaValueType::Enum(name) => write!(f, "Enum<{name}>"),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash, Deserialize, PartialOrd, Ord)]
pub struct Placeholder {
    pub name: Cow<'static, str>,
    pub r#type: PrismaValueType,
}

impl Placeholder {
    pub fn new(name: impl Into<Cow<'static, str>>, r#type: PrismaValueType) -> Self {
        Self {
            name: name.into(),
            r#type,
        }
    }
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
    BASE64_STANDARD.encode(bytes)
}

pub fn decode_bytes(s: impl AsRef<[u8]>) -> PrismaValueResult<Vec<u8>> {
    BASE64_STANDARD
        .decode(s)
        .map_err(|_| ConversionFailure::new("base64 encoded bytes", "PrismaValue::Bytes"))
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

                Some("param") => {
                    let obj = obj
                        .get("prisma__value")
                        .and_then(|v| v.as_object())
                        .ok_or_else(|| ConversionFailure::new("JSON param value", "PrismaValue"))?;

                    let name = obj
                        .get("name")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| ConversionFailure::new("param name", "JSON param value"))?
                        .to_owned();

                    Ok(PrismaValue::Placeholder(Placeholder::new(name, PrismaValueType::Any)))
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
    const JS_MAX_SAFE_INTEGER: u64 = (1u64 << 53) - 1;

    // convert decimals to integers when possible to avoid '.0' formatting
    if let Some(d) = decimal
        .is_integer()
        .then(|| decimal.to_u64())
        .flatten()
        .filter(|&n| n <= JS_MAX_SAFE_INTEGER)
    {
        d.serialize(serializer)
    } else {
        decimal.to_string().parse::<f64>().unwrap().serialize(serializer)
    }
}

fn deserialize_decimal<'de, D>(deserializer: D) -> Result<BigDecimal, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_f64(BigDecimalVisitor)
}

fn serialize_object<S>(obj: &[(String, PrismaValue)], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.collect_map(obj.iter().map(|(k, v)| (k, v)))
}

fn serialize_placeholder<S>(Placeholder { name, r#type }: &Placeholder, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(2))?;

    map.serialize_entry("prisma__type", "param")?;
    map.serialize_entry(
        "prisma__value",
        &json!({
            "name": name,
            "type": r#type.to_string(),
        }),
    )?;

    map.end()
}

fn serialize_generator_call<S>(
    name: &str,
    args: &[PrismaValue],
    return_type: &PrismaValueType,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let mut map = serializer.serialize_map(Some(2))?;

    map.serialize_entry("prisma__type", "generatorCall")?;
    map.serialize_entry(
        "prisma__value",
        &json!({
            "name": name,
            "args": args,
            "returnType": return_type,
        }),
    )?;

    map.end()
}

struct BigDecimalVisitor;

impl serde::de::Visitor<'_> for BigDecimalVisitor {
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

    pub fn placeholder(name: impl Into<Cow<'static, str>>, r#type: PrismaValueType) -> PrismaValue {
        PrismaValue::Placeholder(Placeholder::new(name, r#type))
    }

    pub fn generator_now() -> PrismaValue {
        PrismaValue::GeneratorCall {
            name: "now".into(),
            args: vec![],
            return_type: PrismaValueType::Date,
        }
    }

    pub fn as_boolean(&self) -> Option<&bool> {
        match self {
            PrismaValue::Boolean(bool) => Some(bool),
            _ => None,
        }
    }

    pub fn as_json(&self) -> Option<&String> {
        if let Self::Json(v) = self { Some(v) } else { None }
    }

    pub fn as_tagged(&self) -> TaggedPrismaValue<'_> {
        TaggedPrismaValue::from(self)
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
            PrismaValue::Placeholder(Placeholder { name, r#type }) => write!(f, "var({name}: {type})"),
            PrismaValue::GeneratorCall { name, args, .. } => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{arg}")?;
                }
                write!(f, ")")
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

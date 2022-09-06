use crate::ast::*;
use crate::error::{Error, ErrorKind};

#[cfg(feature = "bigdecimal")]
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
#[cfg(feature = "chrono")]
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
#[cfg(feature = "json")]
use serde_json::{Number, Value as JsonValue};
use std::{
    borrow::{Borrow, Cow},
    convert::TryFrom,
    fmt,
    str::FromStr,
};
#[cfg(feature = "uuid")]
use uuid::Uuid;

/// A value written to the query as-is without parameterization.
#[derive(Debug, Clone, PartialEq)]
pub struct Raw<'a>(pub(crate) Value<'a>);

/// Converts the value into a state to skip parameterization.
///
/// Must be used carefully to avoid SQL injections.
pub trait IntoRaw<'a> {
    fn raw(self) -> Raw<'a>;
}

impl<'a, T> IntoRaw<'a> for T
where
    T: Into<Value<'a>>,
{
    fn raw(self) -> Raw<'a> {
        Raw(self.into())
    }
}

/// A value we must parameterize for the prepared statement. Null values should be
/// defined by their corresponding type variants with a `None` value for best
/// compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    /// 32-bit signed integer.
    Int32(Option<i32>),
    /// 64-bit signed integer.
    Int64(Option<i64>),
    /// 32-bit floating point.
    Float(Option<f32>),
    /// 64-bit floating point.
    Double(Option<f64>),
    /// String value.
    Text(Option<Cow<'a, str>>),
    /// Database enum value.
    Enum(Option<Cow<'a, str>>),
    /// Bytes value.
    Bytes(Option<Cow<'a, [u8]>>),
    /// Boolean value.
    Boolean(Option<bool>),
    /// A single character.
    Char(Option<char>),
    /// An array value (PostgreSQL).
    Array(Option<Vec<Value<'a>>>),
    /// A numeric value.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    Numeric(Option<BigDecimal>),
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
    /// A JSON value.
    Json(Option<serde_json::Value>),
    /// A XML value.
    Xml(Option<Cow<'a, str>>),
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    /// An UUID value.
    Uuid(Option<Uuid>),
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    /// A datetime value.
    DateTime(Option<DateTime<Utc>>),
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    /// A date value.
    Date(Option<NaiveDate>),
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    /// A time value.
    Time(Option<NaiveTime>),
}

pub(crate) struct Params<'a>(pub(crate) &'a [Value<'a>]);

impl<'a> fmt::Display for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();

        write!(f, "[")?;
        for (i, val) in self.0.iter().enumerate() {
            write!(f, "{}", val)?;

            if i < (len - 1) {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl<'a> fmt::Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            Value::Int32(val) => val.map(|v| write!(f, "{}", v)),
            Value::Int64(val) => val.map(|v| write!(f, "{}", v)),
            Value::Float(val) => val.map(|v| write!(f, "{}", v)),
            Value::Double(val) => val.map(|v| write!(f, "{}", v)),
            Value::Text(val) => val.as_ref().map(|v| write!(f, "\"{}\"", v)),
            Value::Bytes(val) => val.as_ref().map(|v| write!(f, "<{} bytes blob>", v.len())),
            Value::Enum(val) => val.as_ref().map(|v| write!(f, "\"{}\"", v)),
            Value::Boolean(val) => val.map(|v| write!(f, "{}", v)),
            Value::Char(val) => val.map(|v| write!(f, "'{}'", v)),
            Value::Array(vals) => vals.as_ref().map(|vals| {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{}", val)?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }),
            Value::Xml(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            #[cfg(feature = "json")]
            Value::Json(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            #[cfg(feature = "uuid")]
            Value::Uuid(val) => val.map(|v| write!(f, "\"{}\"", v)),
            #[cfg(feature = "chrono")]
            Value::DateTime(val) => val.map(|v| write!(f, "\"{}\"", v)),
            #[cfg(feature = "chrono")]
            Value::Date(val) => val.map(|v| write!(f, "\"{}\"", v)),
            #[cfg(feature = "chrono")]
            Value::Time(val) => val.map(|v| write!(f, "\"{}\"", v)),
        };

        match res {
            Some(r) => r,
            None => write!(f, "null"),
        }
    }
}

#[cfg(feature = "json")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        let res = match pv {
            Value::Int32(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            Value::Int64(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            Value::Float(f) => f.map(|f| match Number::from_f64(f as f64) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            Value::Double(f) => f.map(|f| match Number::from_f64(f) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            Value::Text(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Bytes(bytes) => bytes.map(|bytes| serde_json::Value::String(base64::encode(&bytes))),
            Value::Enum(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Boolean(b) => b.map(serde_json::Value::Bool),
            Value::Char(c) => c.map(|c| {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                serde_json::Value::String(s)
            }),
            Value::Xml(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Array(v) => {
                v.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect()))
            }
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(d) => d.map(|d| serde_json::to_value(d.to_f64().unwrap()).unwrap()),
            #[cfg(feature = "json")]
            Value::Json(v) => v,
            #[cfg(feature = "uuid")]
            Value::Uuid(u) => u.map(|u| serde_json::Value::String(u.hyphenated().to_string())),
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => dt.map(|dt| serde_json::Value::String(dt.to_rfc3339())),
            #[cfg(feature = "chrono")]
            Value::Date(date) => date.map(|date| serde_json::Value::String(format!("{}", date))),
            #[cfg(feature = "chrono")]
            Value::Time(time) => time.map(|time| serde_json::Value::String(format!("{}", time))),
        };

        match res {
            Some(val) => val,
            None => serde_json::Value::Null,
        }
    }
}

impl<'a> Value<'a> {
    /// Creates a new 32-bit signed integer.
    pub fn int32<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Value::Int32(Some(value.into()))
    }

    /// Creates a new 64-bit signed integer.
    pub fn int64<I>(value: I) -> Self
    where
        I: Into<i64>,
    {
        Value::Int64(Some(value.into()))
    }

    /// Creates a new 32-bit signed integer.
    pub fn integer<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Value::Int32(Some(value.into()))
    }

    /// Creates a new decimal value.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub const fn numeric(value: BigDecimal) -> Self {
        Value::Numeric(Some(value))
    }

    /// Creates a new float value.
    pub const fn float(value: f32) -> Self {
        Self::Float(Some(value))
    }

    /// Creates a new double value.
    pub const fn double(value: f64) -> Self {
        Self::Double(Some(value))
    }

    /// Creates a new string value.
    pub fn text<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::Text(Some(value.into()))
    }

    /// Creates a new enum value.
    pub fn enum_variant<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::Enum(Some(value.into()))
    }

    /// Creates a new bytes value.
    pub fn bytes<B>(value: B) -> Self
    where
        B: Into<Cow<'a, [u8]>>,
    {
        Value::Bytes(Some(value.into()))
    }

    /// Creates a new boolean value.
    pub fn boolean<B>(value: B) -> Self
    where
        B: Into<bool>,
    {
        Value::Boolean(Some(value.into()))
    }

    /// Creates a new character value.
    pub fn character<C>(value: C) -> Self
    where
        C: Into<char>,
    {
        Value::Char(Some(value.into()))
    }

    /// Creates a new array value.
    pub fn array<I, V>(value: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value<'a>>,
    {
        Value::Array(Some(value.into_iter().map(|v| v.into()).collect()))
    }

    /// Creates a new uuid value.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub const fn uuid(value: Uuid) -> Self {
        Value::Uuid(Some(value))
    }

    /// Creates a new datetime value.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn datetime(value: DateTime<Utc>) -> Self {
        Value::DateTime(Some(value))
    }

    /// Creates a new date value.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn date(value: NaiveDate) -> Self {
        Value::Date(Some(value))
    }

    /// Creates a new time value.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn time(value: NaiveTime) -> Self {
        Value::Time(Some(value))
    }

    /// Creates a new JSON value.
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
    pub const fn json(value: serde_json::Value) -> Self {
        Value::Json(Some(value))
    }

    /// Creates a new XML value.
    pub fn xml<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::Xml(Some(value.into()))
    }

    /// `true` if the `Value` is null.
    pub const fn is_null(&self) -> bool {
        match self {
            Value::Int32(i) => i.is_none(),
            Value::Int64(i) => i.is_none(),
            Value::Float(i) => i.is_none(),
            Value::Double(i) => i.is_none(),
            Value::Text(t) => t.is_none(),
            Value::Enum(e) => e.is_none(),
            Value::Bytes(b) => b.is_none(),
            Value::Boolean(b) => b.is_none(),
            Value::Char(c) => c.is_none(),
            Value::Array(v) => v.is_none(),
            Value::Xml(s) => s.is_none(),
            #[cfg(feature = "bigdecimal")]
            Value::Numeric(r) => r.is_none(),
            #[cfg(feature = "uuid")]
            Value::Uuid(u) => u.is_none(),
            #[cfg(feature = "chrono")]
            Value::DateTime(dt) => dt.is_none(),
            #[cfg(feature = "chrono")]
            Value::Date(d) => d.is_none(),
            #[cfg(feature = "chrono")]
            Value::Time(t) => t.is_none(),
            #[cfg(feature = "json")]
            Value::Json(json) => json.is_none(),
        }
    }

    /// `true` if the `Value` is text.
    pub const fn is_text(&self) -> bool {
        matches!(self, Value::Text(_))
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(Some(cow)) => Some(cow.borrow()),
            Value::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub const fn as_char(&self) -> Option<char> {
        match self {
            Value::Char(c) => *c,
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::Text(Some(cow)) => Some(cow.to_string()),
            Value::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self {
            Value::Text(Some(cow)) => Some(cow.into_owned()),
            Value::Bytes(Some(cow)) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub const fn is_bytes(&self) -> bool {
        matches!(self, Value::Bytes(_))
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Text(Some(cow)) => Some(cow.as_ref().as_bytes()),
            Value::Bytes(Some(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Value::Text(Some(cow)) => Some(cow.to_string().into_bytes()),
            Value::Bytes(Some(cow)) => Some(cow.to_owned().into()),
            _ => None,
        }
    }

    /// `true` if the `Value` is a 32-bit signed integer.
    pub const fn is_i32(&self) -> bool {
        matches!(self, Value::Int32(_))
    }

    /// `true` if the `Value` is a 64-bit signed integer.
    pub const fn is_i64(&self) -> bool {
        matches!(self, Value::Int64(_))
    }

    /// `true` if the `Value` is a signed integer.
    pub const fn is_integer(&self) -> bool {
        matches!(self, Value::Int32(_) | Value::Int64(_))
    }

    /// Returns an `i64` if the value is a 64-bit signed integer, otherwise `None`.
    pub const fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i32` if the value is a 32-bit signed integer, otherwise `None`.
    pub const fn as_i32(&self) -> Option<i32> {
        match self {
            Value::Int32(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i64` if the value is a signed integer, otherwise `None`.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Int32(i) => i.map(|i| i as i64),
            Value::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns a `f64` if the value is a double, otherwise `None`.
    pub const fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Double(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// Returns a `f32` if the value is a double, otherwise `None`.
    pub const fn as_f32(&self) -> Option<f32> {
        match self {
            Value::Float(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// `true` if the `Value` is a numeric value or can be converted to one.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Value::Numeric(_) | Value::Float(_) | Value::Double(_))
    }

    /// Returns a bigdecimal, if the value is a numeric, float or double value,
    /// otherwise `None`.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn into_numeric(self) -> Option<BigDecimal> {
        match self {
            Value::Numeric(d) => d,
            Value::Float(f) => f.and_then(BigDecimal::from_f32),
            Value::Double(f) => f.and_then(BigDecimal::from_f64),
            _ => None,
        }
    }

    /// Returns a reference to a bigdecimal, if the value is a numeric.
    /// Otherwise `None`.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub const fn as_numeric(&self) -> Option<&BigDecimal> {
        match self {
            Value::Numeric(d) => d.as_ref(),
            _ => None,
        }
    }

    /// `true` if the `Value` is a boolean value.
    pub const fn is_bool(&self) -> bool {
        match self {
            Value::Boolean(_) => true,
            // For schemas which don't tag booleans
            Value::Int32(Some(i)) if *i == 0 || *i == 1 => true,
            Value::Int64(Some(i)) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => *b,
            // For schemas which don't tag booleans
            Value::Int32(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            Value::Int64(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// `true` if the `Value` is an Array.
    pub const fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// `true` if the `Value` is of UUID type.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub const fn is_uuid(&self) -> bool {
        matches!(self, Value::Uuid(_))
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub const fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Value::Uuid(u) => *u,
            _ => None,
        }
    }

    /// `true` if the `Value` is a DateTime.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn is_datetime(&self) -> bool {
        matches!(self, Value::DateTime(_))
    }

    /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match self {
            Value::DateTime(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a Date.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn is_date(&self) -> bool {
        matches!(self, Value::Date(_))
    }

    /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn as_date(&self) -> Option<NaiveDate> {
        match self {
            Value::Date(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a `Time`.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn is_time(&self) -> bool {
        matches!(self, Value::Time(_))
    }

    /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
    #[cfg(feature = "chrono")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
    pub const fn as_time(&self) -> Option<NaiveTime> {
        match self {
            Value::Time(time) => *time,
            _ => None,
        }
    }

    /// `true` if the `Value` is a JSON value.
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
    pub const fn is_json(&self) -> bool {
        matches!(self, Value::Json(_))
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
    pub const fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Value::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
    pub fn into_json(self) -> Option<serde_json::Value> {
        match self {
            Value::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Returns a Vec<T> if the value is an array of T, otherwise `None`.
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<Value>
        T: TryFrom<Value<'a>>,
    {
        match self {
            Value::Array(Some(vec)) => {
                let rslt: Result<Vec<_>, _> = vec.into_iter().map(T::try_from).collect();
                match rslt {
                    Err(_) => None,
                    Ok(values) => Some(values),
                }
            }
            _ => None,
        }
    }
}

value!(val: i64, Int64, val);
value!(val: i32, Int32, val);
value!(val: bool, Boolean, val);
value!(val: &'a str, Text, val.into());
value!(val: String, Text, val.into());
value!(val: usize, Int64, i64::try_from(val).unwrap());
value!(val: &'a [u8], Bytes, val.into());
value!(val: f64, Double, val);
value!(val: f32, Float, val);

#[cfg(feature = "chrono")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
value!(val: DateTime<Utc>, DateTime, val);
#[cfg(feature = "chrono")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
value!(val: chrono::NaiveTime, Time, val);
#[cfg(feature = "chrono")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
value!(val: chrono::NaiveDate, Date, val);
#[cfg(feature = "bigdecimal")]
value!(val: BigDecimal, Numeric, val);
#[cfg(feature = "json")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "json")))]
value!(val: JsonValue, Json, val);
#[cfg(feature = "uuid")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
value!(val: Uuid, Uuid, val);

impl<'a> TryFrom<Value<'a>> for i64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i64, Self::Error> {
        value
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for i32 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i32, Self::Error> {
        value
            .as_i32()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i32")).build())
    }
}

#[cfg(feature = "bigdecimal")]
impl<'a> TryFrom<Value<'a>> for BigDecimal {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<BigDecimal, Self::Error> {
        value
            .into_numeric()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a decimal")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for f64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<f64, Self::Error> {
        value
            .as_f64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a f64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for String {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<String, Self::Error> {
        value
            .into_string()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a string")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<bool, Self::Error> {
        value
            .as_bool()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a bool")).build())
    }
}

#[cfg(feature = "chrono")]
#[cfg_attr(feature = "docs", doc(cfg(feature = "chrono")))]
impl<'a> TryFrom<Value<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a datetime")).build())
    }
}

impl<'a> TryFrom<&Value<'a>> for Option<std::net::IpAddr> {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Option<std::net::IpAddr>, Self::Error> {
        match value {
            val @ Value::Text(Some(_)) => {
                let text = val.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            val @ Value::Bytes(Some(_)) => {
                let text = val.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            v if v.is_null() => Ok(None),
            v => {
                let kind =
                    ErrorKind::conversion(format!("Couldn't convert value of type `{:?}` to std::net::IpAddr.", v));

                Err(Error::builder(kind).build())
            }
        }
    }
}

#[cfg(feature = "uuid")]
impl<'a> TryFrom<&Value<'a>> for Option<uuid::Uuid> {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Option<uuid::Uuid>, Self::Error> {
        match value {
            Value::Uuid(uuid) => Ok(*uuid),
            val @ Value::Text(Some(_)) => {
                let text = val.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            val @ Value::Bytes(Some(_)) => {
                let text = val.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            v if v.is_null() => Ok(None),
            v => {
                let kind = ErrorKind::conversion(format!("Couldn't convert value of type `{:?}` to uuid::Uuid.", v));

                Err(Error::builder(kind).build())
            }
        }
    }
}

/// An in-memory temporary table. Can be used in some of the databases in a
/// place of an actual table. Doesn't work in MySQL 5.7.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Values<'a> {
    pub(crate) rows: Vec<Row<'a>>,
}

impl<'a> Values<'a> {
    /// Create a new empty in-memory set of values.
    pub fn empty() -> Self {
        Self { rows: Vec::new() }
    }

    /// Create a new in-memory set of values.
    pub fn new(rows: Vec<Row<'a>>) -> Self {
        Self { rows }
    }

    /// Create a new in-memory set of values with an allocated capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            rows: Vec::with_capacity(capacity),
        }
    }

    /// Add value to the temporary table.
    pub fn push<T>(&mut self, row: T)
    where
        T: Into<Row<'a>>,
    {
        self.rows.push(row.into());
    }

    /// The number of rows in the in-memory table.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// True if has no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn row_len(&self) -> usize {
        match self.rows.split_first() {
            Some((row, _)) => row.len(),
            None => 0,
        }
    }

    pub fn flatten_row(self) -> Option<Row<'a>> {
        let mut result = Row::with_capacity(self.len());

        for mut row in self.rows.into_iter() {
            match row.pop() {
                Some(value) => result.push(value),
                None => return None,
            }
        }

        Some(result)
    }
}

impl<'a, I, R> From<I> for Values<'a>
where
    I: Iterator<Item = R>,
    R: Into<Row<'a>>,
{
    fn from(rows: I) -> Self {
        Self {
            rows: rows.map(|r| r.into()).collect(),
        }
    }
}

impl<'a> IntoIterator for Values<'a> {
    type Item = Row<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.rows.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "chrono")]
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints32_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1]);
        let values: Vec<i32> = pv.into_vec().expect("convert into Vec<i32>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_ints64_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1_i64]);
        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1.0]);
        let values: Vec<f64> = pv.into_vec().expect("convert into Vec<f64>");
        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = Value::array(vec!["test"]);
        let values: Vec<String> = pv.into_vec().expect("convert into Vec<String>");
        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![true]);
        let values: Vec<bool> = pv.into_vec().expect("convert into Vec<bool>");
        assert_eq!(values, vec![true]);
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = Value::array(vec![datetime]);
        let values: Vec<DateTime<Utc>> = pv.into_vec().expect("convert into Vec<DateTime>");
        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = Value::array(vec![1]);
        let rslt: Option<Vec<f64>> = pv.into_vec();
        assert!(rslt.is_none());
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn display_format_for_datetime() {
        let dt: DateTime<Utc> = DateTime::from_str("2019-07-27T05:30:30Z").expect("failed while parsing date");
        let pv = Value::datetime(dt);

        assert_eq!(format!("{}", pv), "\"2019-07-27 05:30:30 UTC\"");
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn display_format_for_date() {
        let date = NaiveDate::from_ymd(2022, 8, 11);
        let pv = Value::date(date);

        assert_eq!(format!("{}", pv), "\"2022-08-11\"");
    }

    #[test]
    #[cfg(feature = "chrono")]
    fn display_format_for_time() {
        let time = NaiveTime::from_hms(16, 17, 00);
        let pv = Value::time(time);

        assert_eq!(format!("{}", pv), "\"16:17:00\"");
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn display_format_for_uuid() {
        let id = Uuid::from_str("67e5504410b1426f9247bb680e5fe0c8").unwrap();
        let pv = Value::uuid(id);

        assert_eq!(format!("{}", pv), "\"67e55044-10b1-426f-9247-bb680e5fe0c8\"");
    }
}

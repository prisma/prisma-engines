use crate::ast::*;
use crate::error::{Error, ErrorKind};

#[cfg(feature = "bigdecimal")]
use bigdecimal::{BigDecimal, FromPrimitive, ToPrimitive};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde_json::{Number, Value as JsonValue};
use std::fmt::Display;
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
pub struct Raw<'a>(pub(crate) ValueInner<'a>);

/// Converts the value into a state to skip parameterization.
///
/// Must be used carefully to avoid SQL injections.
pub trait IntoRaw<'a> {
    fn raw(self) -> Raw<'a>;
}

impl<'a, T> IntoRaw<'a> for T
where
    T: Into<ValueInner<'a>>,
{
    fn raw(self) -> Raw<'a> {
        Raw(self.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value<'a> {
    pub inner: ValueInner<'a>,
    pub native_column_type: Option<Cow<'a, str>>,
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<'a> From<ValueInner<'a>> for Value<'a> {
    fn from(inner: ValueInner<'a>) -> Self {
        Self {
            inner,
            native_column_type: None,
        }
    }
}

/// A value we must parameterize for the prepared statement. Null values should be
/// defined by their corresponding type variants with a `None` value for best
/// compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueInner<'a> {
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
    /// The optional `EnumName` is only used on PostgreSQL.
    /// Read more about it here: https://github.com/prisma/prisma-engines/pull/4280
    Enum(Option<EnumVariant<'a>>, Option<EnumName<'a>>),
    /// Database enum array (PostgreSQL specific).
    /// We use a different variant than `ValueInner::Array` to uplift the `EnumName`
    /// and have it available even for empty enum arrays.
    EnumArray(Option<Vec<EnumVariant<'a>>>, Option<EnumName<'a>>),
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
    /// A JSON value.
    Json(Option<serde_json::Value>),
    /// A XML value.
    Xml(Option<Cow<'a, str>>),
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    /// An UUID value.
    Uuid(Option<Uuid>),
    /// A datetime value.
    DateTime(Option<DateTime<Utc>>),
    /// A date value.
    Date(Option<NaiveDate>),
    /// A time value.
    Time(Option<NaiveTime>),
}

pub(crate) struct Params<'a>(pub(crate) &'a [Value<'a>]);

impl<'a> Display for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();

        write!(f, "[")?;
        for (i, val) in self.0.iter().enumerate() {
            write!(f, "{val}")?;

            if i < (len - 1) {
                write!(f, ",")?;
            }
        }
        write!(f, "]")
    }
}

impl<'a> fmt::Display for ValueInner<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            ValueInner::Int32(val) => val.map(|v| write!(f, "{v}")),
            ValueInner::Int64(val) => val.map(|v| write!(f, "{v}")),
            ValueInner::Float(val) => val.map(|v| write!(f, "{v}")),
            ValueInner::Double(val) => val.map(|v| write!(f, "{v}")),
            ValueInner::Text(val) => val.as_ref().map(|v| write!(f, "\"{v}\"")),
            ValueInner::Bytes(val) => val.as_ref().map(|v| write!(f, "<{} bytes blob>", v.len())),
            ValueInner::Enum(val, _) => val.as_ref().map(|v| write!(f, "\"{v}\"")),
            ValueInner::EnumArray(vals, _) => vals.as_ref().map(|vals| {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{val}")?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }),
            ValueInner::Boolean(val) => val.map(|v| write!(f, "{v}")),
            ValueInner::Char(val) => val.map(|v| write!(f, "'{v}'")),
            ValueInner::Array(vals) => vals.as_ref().map(|vals| {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{val}")?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }),
            ValueInner::Xml(val) => val.as_ref().map(|v| write!(f, "{v}")),
            #[cfg(feature = "bigdecimal")]
            ValueInner::Numeric(val) => val.as_ref().map(|v| write!(f, "{v}")),
            ValueInner::Json(val) => val.as_ref().map(|v| write!(f, "{v}")),
            #[cfg(feature = "uuid")]
            ValueInner::Uuid(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueInner::DateTime(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueInner::Date(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueInner::Time(val) => val.map(|v| write!(f, "\"{v}\"")),
        };

        match res {
            Some(r) => r,
            None => write!(f, "null"),
        }
    }
}

impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        pv.inner.into()
    }
}

impl<'a> From<ValueInner<'a>> for serde_json::Value {
    fn from(pv: ValueInner<'a>) -> Self {
        let res = match pv {
            ValueInner::Int32(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            ValueInner::Int64(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            ValueInner::Float(f) => f.map(|f| match Number::from_f64(f as f64) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            ValueInner::Double(f) => f.map(|f| match Number::from_f64(f) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            ValueInner::Text(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueInner::Bytes(bytes) => bytes.map(|bytes| serde_json::Value::String(base64::encode(bytes))),
            ValueInner::Enum(cow, _) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueInner::EnumArray(values, _) => values.map(|values| {
                serde_json::Value::Array(
                    values
                        .into_iter()
                        .map(|value| serde_json::Value::String(value.into_owned()))
                        .collect(),
                )
            }),
            ValueInner::Boolean(b) => b.map(serde_json::Value::Bool),
            ValueInner::Char(c) => c.map(|c| {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                serde_json::Value::String(s)
            }),
            ValueInner::Xml(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueInner::Array(v) => {
                v.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect()))
            }
            #[cfg(feature = "bigdecimal")]
            ValueInner::Numeric(d) => d.map(|d| serde_json::to_value(d.to_f64().unwrap()).unwrap()),
            ValueInner::Json(v) => v,
            #[cfg(feature = "uuid")]
            ValueInner::Uuid(u) => u.map(|u| serde_json::Value::String(u.hyphenated().to_string())),
            ValueInner::DateTime(dt) => dt.map(|dt| serde_json::Value::String(dt.to_rfc3339())),
            ValueInner::Date(date) => date.map(|date| serde_json::Value::String(format!("{date}"))),
            ValueInner::Time(time) => time.map(|time| serde_json::Value::String(format!("{time}"))),
        };

        match res {
            Some(val) => val,
            None => serde_json::Value::Null,
        }
    }
}

impl<'a> ValueInner<'a> {
    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        match self {
            ValueInner::Int32(i) => i.is_none(),
            ValueInner::Int64(i) => i.is_none(),
            ValueInner::Float(i) => i.is_none(),
            ValueInner::Double(i) => i.is_none(),
            ValueInner::Text(t) => t.is_none(),
            ValueInner::Enum(e, _) => e.is_none(),
            ValueInner::EnumArray(e, _) => e.is_none(),
            ValueInner::Bytes(b) => b.is_none(),
            ValueInner::Boolean(b) => b.is_none(),
            ValueInner::Char(c) => c.is_none(),
            ValueInner::Array(v) => v.is_none(),
            ValueInner::Xml(s) => s.is_none(),
            #[cfg(feature = "bigdecimal")]
            ValueInner::Numeric(r) => r.is_none(),
            #[cfg(feature = "uuid")]
            ValueInner::Uuid(u) => u.is_none(),
            ValueInner::DateTime(dt) => dt.is_none(),
            ValueInner::Date(d) => d.is_none(),
            ValueInner::Time(t) => t.is_none(),
            ValueInner::Json(json) => json.is_none(),
        }
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
        matches!(self, ValueInner::Text(_))
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        matches!(self, ValueInner::Bytes(_))
    }
    /// `true` if the `Value` is a 32-bit signed integer.
    pub fn is_i32(&self) -> bool {
        matches!(self, ValueInner::Int32(_))
    }

    /// `true` if the `Value` is a 64-bit signed integer.
    pub fn is_i64(&self) -> bool {
        matches!(self, ValueInner::Int64(_))
    }

    /// `true` if the `Value` is a signed integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, ValueInner::Int32(_) | ValueInner::Int64(_))
    }

    /// `true` if the `Value` is a numeric value or can be converted to one.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            ValueInner::Numeric(_) | ValueInner::Float(_) | ValueInner::Double(_)
        )
    }

    /// `true` if the `Value` is a boolean value.
    pub fn is_bool(&self) -> bool {
        match self {
            ValueInner::Boolean(_) => true,
            // For schemas which don't tag booleans
            ValueInner::Int32(Some(i)) if *i == 0 || *i == 1 => true,
            ValueInner::Int64(Some(i)) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }
    /// `true` if the `Value` is an Array.
    pub fn is_array(&self) -> bool {
        matches!(self, ValueInner::Array(_))
    }

    /// `true` if the `Value` is of UUID type.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn is_uuid(&self) -> bool {
        matches!(self, ValueInner::Uuid(_))
    }

    /// `true` if the `Value` is a Date.
    pub fn is_date(&self) -> bool {
        matches!(self, ValueInner::Date(_))
    }

    /// `true` if the `Value` is a DateTime.
    pub fn is_datetime(&self) -> bool {
        matches!(self, ValueInner::DateTime(_))
    }

    /// `true` if the `Value` is a Time.
    pub fn is_time(&self) -> bool {
        matches!(self, ValueInner::Time(_))
    }

    /// `true` if the `Value` is a JSON value.
    pub fn is_json(&self) -> bool {
        matches!(self, ValueInner::Json(_))
    }
}

impl<'a> Value<'a> {
    /// Creates a new 32-bit signed integer.
    pub fn int32<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Value::from(ValueInner::Int32(Some(value.into())))
    }

    /// Creates a new 64-bit signed integer.
    pub fn int64<I>(value: I) -> Self
    where
        I: Into<i64>,
    {
        Value::from(ValueInner::Int64(Some(value.into())))
    }

    /// Creates a new 32-bit signed integer.
    pub fn integer<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Value::from(ValueInner::Int32(Some(value.into())))
    }

    /// Creates a new decimal value.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn numeric(value: BigDecimal) -> Self {
        Value::from(ValueInner::Numeric(Some(value)))
    }

    /// Creates a new float value.
    pub fn float(value: f32) -> Self {
        Value::from(ValueInner::Float(Some(value)))
    }

    /// Creates a new double value.
    pub fn double(value: f64) -> Self {
        Value::from(ValueInner::Double(Some(value)))
    }

    /// Creates a new string value.
    pub fn text<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::from(ValueInner::Text(Some(value.into())))
    }

    /// Creates a new enum value.
    pub fn enum_variant<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::from(ValueInner::Enum(Some(EnumVariant::new(value)), None))
    }

    /// Creates a new enum value with the name of the enum attached.
    pub fn enum_variant_with_name<T, U, V>(value: T, name: U, schema_name: Option<V>) -> Self
    where
        T: Into<Cow<'a, str>>,
        U: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        Value::from(ValueInner::Enum(
            Some(EnumVariant::new(value)),
            Some(EnumName::new(name, schema_name)),
        ))
    }

    /// Creates a new bytes value.
    pub fn bytes<B>(value: B) -> Self
    where
        B: Into<Cow<'a, [u8]>>,
    {
        Value::from(ValueInner::Bytes(Some(value.into())))
    }

    /// Creates a new boolean value.
    pub fn boolean<B>(value: B) -> Self
    where
        B: Into<bool>,
    {
        Value::from(ValueInner::Boolean(Some(value.into())))
    }

    /// Creates a new character value.
    pub fn character<C>(value: C) -> Self
    where
        C: Into<char>,
    {
        Value::from(ValueInner::Char(Some(value.into())))
    }

    /// Creates a new array value.
    pub fn array<I, V>(value: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value<'a>>,
    {
        Value::from(ValueInner::Array(Some(
            value.into_iter().map(|v| Value::from(v.into())).collect(),
        )))
    }

    /// Creates a new uuid value.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn uuid(value: Uuid) -> Self {
        Value::from(ValueInner::Uuid(Some(value)))
    }

    /// Creates a new datetime value.
    pub fn datetime(value: DateTime<Utc>) -> Self {
        Value::from(ValueInner::DateTime(Some(value)))
    }

    /// Creates a new date value.
    pub fn date(value: NaiveDate) -> Self {
        Value::from(ValueInner::Date(Some(value)))
    }

    /// Creates a new time value.
    pub fn time(value: NaiveTime) -> Self {
        Value::from(ValueInner::Time(Some(value)))
    }

    /// Creates a new JSON value.
    pub fn json(value: serde_json::Value) -> Self {
        Value::from(ValueInner::Json(Some(value)))
    }

    /// Creates a new XML value.
    pub fn xml<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::from(ValueInner::Xml(Some(value.into())))
    }

    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        self.inner.is_null()
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
        self.inner.is_text()
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match &self.inner {
            ValueInner::Text(Some(cow)) => Some(cow.borrow()),
            ValueInner::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub fn as_char(&self) -> Option<char> {
        match &self.inner {
            ValueInner::Char(c) => *c,
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match &self.inner {
            ValueInner::Text(Some(cow)) => Some(cow.to_string()),
            ValueInner::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self.inner {
            ValueInner::Text(Some(cow)) => Some(cow.into_owned()),
            ValueInner::Bytes(Some(cow)) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        self.inner.is_bytes()
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.inner {
            ValueInner::Text(Some(cow)) => Some(cow.as_ref().as_bytes()),
            ValueInner::Bytes(Some(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match &self.inner {
            ValueInner::Text(Some(cow)) => Some(cow.to_string().into_bytes()),
            ValueInner::Bytes(Some(cow)) => Some(cow.to_vec()),
            _ => None,
        }
    }

    /// `true` if the `Value` is a 32-bit signed integer.
    pub fn is_i32(&self) -> bool {
        self.inner.is_i32()
    }

    /// `true` if the `Value` is a 64-bit signed integer.
    pub fn is_i64(&self) -> bool {
        self.inner.is_i64()
    }

    /// `true` if the `Value` is a signed integer.
    pub fn is_integer(&self) -> bool {
        self.inner.is_integer()
    }

    /// Returns an `i64` if the value is a 64-bit signed integer, otherwise `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match &self.inner {
            ValueInner::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i32` if the value is a 32-bit signed integer, otherwise `None`.
    pub fn as_i32(&self) -> Option<i32> {
        match &self.inner {
            ValueInner::Int32(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i64` if the value is a signed integer, otherwise `None`.
    pub fn as_integer(&self) -> Option<i64> {
        match &self.inner {
            ValueInner::Int32(i) => i.map(|i| i as i64),
            ValueInner::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns a `f64` if the value is a double, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match &self.inner {
            ValueInner::Double(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// Returns a `f32` if the value is a double, otherwise `None`.
    pub fn as_f32(&self) -> Option<f32> {
        match &self.inner {
            ValueInner::Float(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// `true` if the `Value` is a numeric value or can be converted to one.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn is_numeric(&self) -> bool {
        self.inner.is_numeric()
    }

    /// Returns a bigdecimal, if the value is a numeric, float or double value,
    /// otherwise `None`.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn into_numeric(self) -> Option<BigDecimal> {
        match self.inner {
            ValueInner::Numeric(d) => d,
            ValueInner::Float(f) => f.and_then(BigDecimal::from_f32),
            ValueInner::Double(f) => f.and_then(BigDecimal::from_f64),
            _ => None,
        }
    }

    /// Returns a reference to a bigdecimal, if the value is a numeric.
    /// Otherwise `None`.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn as_numeric(&self) -> Option<&BigDecimal> {
        match &self.inner {
            ValueInner::Numeric(d) => d.as_ref(),
            _ => None,
        }
    }

    /// `true` if the `Value` is a boolean value.
    pub fn is_bool(&self) -> bool {
        self.inner.is_bool()
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match &self.inner {
            ValueInner::Boolean(b) => *b,
            // For schemas which don't tag booleans
            ValueInner::Int32(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            ValueInner::Int64(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// `true` if the `Value` is an Array.
    pub fn is_array(&self) -> bool {
        self.inner.is_array()
    }

    /// `true` if the `Value` is of UUID type.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn is_uuid(&self) -> bool {
        self.inner.is_uuid()
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match &self.inner {
            ValueInner::Uuid(u) => *u,
            _ => None,
        }
    }

    /// `true` if the `Value` is a DateTime.
    pub fn is_datetime(&self) -> bool {
        self.inner.is_datetime()
    }

    /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match &self.inner {
            ValueInner::DateTime(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a Date.
    pub fn is_date(&self) -> bool {
        self.inner.is_date()
    }

    /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
    pub fn as_date(&self) -> Option<NaiveDate> {
        match &self.inner {
            ValueInner::Date(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a `Time`.
    pub fn is_time(&self) -> bool {
        self.inner.is_time()
    }

    /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
    pub fn as_time(&self) -> Option<NaiveTime> {
        match &self.inner {
            ValueInner::Time(time) => *time,
            _ => None,
        }
    }

    /// `true` if the `Value` is a JSON value.
    pub fn is_json(&self) -> bool {
        self.inner.is_json()
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match &self.inner {
            ValueInner::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    pub fn into_json(self) -> Option<serde_json::Value> {
        match self.inner {
            ValueInner::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Returns a `Vec<T>` if the value is an array of `T`, otherwise `None`.
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<Value>
        T: TryFrom<ValueInner<'a>>,
    {
        match self.inner {
            ValueInner::Array(Some(vec)) => {
                let rslt: Result<Vec<_>, _> = vec.into_iter().map(|val| val.inner.try_into()).collect();
                match rslt {
                    Err(_) => None,
                    Ok(values) => Some(values),
                }
            }
            _ => None,
        }
    }

    /// Returns a cloned Vec<T> if the value is an array of T, otherwise `None`.
    pub fn to_vec<T>(&self) -> Option<Vec<T>>
    where
        T: TryFrom<Value<'a>>,
    {
        match &self.inner {
            ValueInner::Array(Some(vec)) => {
                let rslt: Result<Vec<_>, _> = vec.clone().into_iter().map(T::try_from).collect();
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

value!(val: DateTime<Utc>, DateTime, val);
value!(val: chrono::NaiveTime, Time, val);
value!(val: chrono::NaiveDate, Date, val);
#[cfg(feature = "bigdecimal")]
value!(val: BigDecimal, Numeric, val);
value!(val: JsonValue, Json, val);
#[cfg(feature = "uuid")]
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
        match &value.inner {
            val @ ValueInner::Text(Some(_)) => {
                let text = value.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            val @ ValueInner::Bytes(Some(_)) => {
                let text = value.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            _ if value.is_null() => Ok(None),
            v => {
                let kind =
                    ErrorKind::conversion(format!("Couldn't convert value of type `{v:?}` to std::net::IpAddr."));

                Err(Error::builder(kind).build())
            }
        }
    }
}

#[cfg(feature = "uuid")]
impl<'a> TryFrom<&Value<'a>> for Option<uuid::Uuid> {
    type Error = Error;

    fn try_from(value: &Value<'a>) -> Result<Option<uuid::Uuid>, Self::Error> {
        match &value.inner {
            ValueInner::Uuid(uuid) => Ok(*uuid),
            val @ ValueInner::Text(Some(_)) => {
                let text = value.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            val @ ValueInner::Bytes(Some(_)) => {
                let text = value.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            v if value.is_null() => Ok(None),
            v => {
                let kind = ErrorKind::conversion(format!("Couldn't convert value of type `{v:?}` to uuid::Uuid."));

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
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints32_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueInner::array(vec![1]));
        let values: Vec<i32> = pv.into_vec().expect("convert into Vec<i32>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_ints64_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueInner::array(vec![1_i64]));
        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueInner::array(vec![1.0]));
        let values: Vec<f64> = pv.into_vec().expect("convert into Vec<f64>");
        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueInner::array(vec!["test"]));
        let values: Vec<String> = pv.into_vec().expect("convert into Vec<String>");
        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueInner::array(vec![true]));
        let values: Vec<bool> = pv.into_vec().expect("convert into Vec<bool>");
        assert_eq!(values, vec![true]);
    }

    #[test]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = Value::from(ValueInner::array(vec![datetime]));
        let values: Vec<DateTime<Utc>> = pv.into_vec().expect("convert into Vec<DateTime>");
        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = Value::from(ValueInner::array(vec![1]));
        let rslt: Option<Vec<f64>> = pv.into_vec();
        assert!(rslt.is_none());
    }

    #[test]
    fn display_format_for_datetime() {
        let dt: DateTime<Utc> = DateTime::from_str("2019-07-27T05:30:30Z").expect("failed while parsing date");
        let pv = Value::from(ValueInner::datetime(dt));

        assert_eq!(format!("{pv}"), "\"2019-07-27 05:30:30 UTC\"");
    }

    #[test]
    fn display_format_for_date() {
        let date = NaiveDate::from_ymd_opt(2022, 8, 11).unwrap();
        let pv = Value::from(ValueInner::date(date));

        assert_eq!(format!("{pv}"), "\"2022-08-11\"");
    }

    #[test]
    fn display_format_for_time() {
        let time = NaiveTime::from_hms_opt(16, 17, 00).unwrap();
        let pv = Value::from(ValueInner::time(time));

        assert_eq!(format!("{pv}"), "\"16:17:00\"");
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn display_format_for_uuid() {
        let id = Uuid::from_str("67e5504410b1426f9247bb680e5fe0c8").unwrap();
        let pv = Value::from(ValueInner::uuid(id));

        assert_eq!(format!("{pv}"), "\"67e55044-10b1-426f-9247-bb680e5fe0c8\"");
    }
}

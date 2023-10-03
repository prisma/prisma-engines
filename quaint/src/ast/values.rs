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
pub struct Raw<'a>(pub(crate) ValueType<'a>);

/// Converts the value into a state to skip parameterization.
///
/// Must be used carefully to avoid SQL injections.
pub trait IntoRaw<'a> {
    fn raw(self) -> Raw<'a>;
}

impl<'a, T> IntoRaw<'a> for T
where
    T: Into<ValueType<'a>>,
{
    fn raw(self) -> Raw<'a> {
        Raw(self.into())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Value<'a> {
    pub typed: ValueType<'a>,
    pub native_column_type: Option<Cow<'a, str>>,
}

impl<'a> Display for Value<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.typed.fmt(f)
    }
}

impl<'a> From<ValueType<'a>> for Value<'a> {
    fn from(inner: ValueType<'a>) -> Self {
        Self {
            typed: inner,
            native_column_type: None,
        }
    }
}

impl<'a> Into<ValueType<'a>> for Value<'a> {
    fn into(self) -> ValueType<'a> {
        return self.typed;
    }
}

/// A value we must parameterize for the prepared statement. Null values should be
/// defined by their corresponding type variants with a `None` value for best
/// compatibility.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueType<'a> {
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

impl<'a> fmt::Display for ValueType<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let res = match self {
            ValueType::Int32(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Int64(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Float(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Double(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Text(val) => val.as_ref().map(|v| write!(f, "\"{v}\"")),
            ValueType::Bytes(val) => val.as_ref().map(|v| write!(f, "<{} bytes blob>", v.len())),
            ValueType::Enum(val, _) => val.as_ref().map(|v| write!(f, "\"{v}\"")),
            ValueType::EnumArray(vals, _) => vals.as_ref().map(|vals| {
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
            ValueType::Boolean(val) => val.map(|v| write!(f, "{v}")),
            ValueType::Char(val) => val.map(|v| write!(f, "'{v}'")),
            ValueType::Array(vals) => vals.as_ref().map(|vals| {
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
            ValueType::Xml(val) => val.as_ref().map(|v| write!(f, "{v}")),
            #[cfg(feature = "bigdecimal")]
            ValueType::Numeric(val) => val.as_ref().map(|v| write!(f, "{v}")),
            ValueType::Json(val) => val.as_ref().map(|v| write!(f, "{v}")),
            #[cfg(feature = "uuid")]
            ValueType::Uuid(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueType::DateTime(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueType::Date(val) => val.map(|v| write!(f, "\"{v}\"")),
            ValueType::Time(val) => val.map(|v| write!(f, "\"{v}\"")),
        };

        match res {
            Some(r) => r,
            None => write!(f, "null"),
        }
    }
}

impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        pv.typed.into()
    }
}

impl<'a> From<ValueType<'a>> for serde_json::Value {
    fn from(pv: ValueType<'a>) -> Self {
        let res = match pv {
            ValueType::Int32(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            ValueType::Int64(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            ValueType::Float(f) => f.map(|f| match Number::from_f64(f as f64) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            ValueType::Double(f) => f.map(|f| match Number::from_f64(f) {
                Some(number) => serde_json::Value::Number(number),
                None => serde_json::Value::Null,
            }),
            ValueType::Text(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueType::Bytes(bytes) => bytes.map(|bytes| serde_json::Value::String(base64::encode(bytes))),
            ValueType::Enum(cow, _) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueType::EnumArray(values, _) => values.map(|values| {
                serde_json::Value::Array(
                    values
                        .into_iter()
                        .map(|value| serde_json::Value::String(value.into_owned()))
                        .collect(),
                )
            }),
            ValueType::Boolean(b) => b.map(serde_json::Value::Bool),
            ValueType::Char(c) => c.map(|c| {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                serde_json::Value::String(s)
            }),
            ValueType::Xml(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            ValueType::Array(v) => {
                v.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect()))
            }
            #[cfg(feature = "bigdecimal")]
            ValueType::Numeric(d) => d.map(|d| serde_json::to_value(d.to_f64().unwrap()).unwrap()),
            ValueType::Json(v) => v,
            #[cfg(feature = "uuid")]
            ValueType::Uuid(u) => u.map(|u| serde_json::Value::String(u.hyphenated().to_string())),
            ValueType::DateTime(dt) => dt.map(|dt| serde_json::Value::String(dt.to_rfc3339())),
            ValueType::Date(date) => date.map(|date| serde_json::Value::String(format!("{date}"))),
            ValueType::Time(time) => time.map(|time| serde_json::Value::String(format!("{time}"))),
        };

        match res {
            Some(val) => val,
            None => serde_json::Value::Null,
        }
    }
}

impl<'a> ValueType<'a> {
    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        match self {
            ValueType::Int32(i) => i.is_none(),
            ValueType::Int64(i) => i.is_none(),
            ValueType::Float(i) => i.is_none(),
            ValueType::Double(i) => i.is_none(),
            ValueType::Text(t) => t.is_none(),
            ValueType::Enum(e, _) => e.is_none(),
            ValueType::EnumArray(e, _) => e.is_none(),
            ValueType::Bytes(b) => b.is_none(),
            ValueType::Boolean(b) => b.is_none(),
            ValueType::Char(c) => c.is_none(),
            ValueType::Array(v) => v.is_none(),
            ValueType::Xml(s) => s.is_none(),
            #[cfg(feature = "bigdecimal")]
            ValueType::Numeric(r) => r.is_none(),
            #[cfg(feature = "uuid")]
            ValueType::Uuid(u) => u.is_none(),
            ValueType::DateTime(dt) => dt.is_none(),
            ValueType::Date(d) => d.is_none(),
            ValueType::Time(t) => t.is_none(),
            ValueType::Json(json) => json.is_none(),
        }
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
        matches!(self, ValueType::Text(_))
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        matches!(self, ValueType::Bytes(_))
    }
    /// `true` if the `Value` is a 32-bit signed integer.
    pub fn is_i32(&self) -> bool {
        matches!(self, ValueType::Int32(_))
    }

    /// `true` if the `Value` is a 64-bit signed integer.
    pub fn is_i64(&self) -> bool {
        matches!(self, ValueType::Int64(_))
    }

    /// `true` if the `Value` is a signed integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, ValueType::Int32(_) | ValueType::Int64(_))
    }

    /// `true` if the `Value` is a numeric value or can be converted to one.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn is_numeric(&self) -> bool {
        matches!(self, ValueType::Numeric(_) | ValueType::Float(_) | ValueType::Double(_))
    }

    /// `true` if the `Value` is a boolean value.
    pub fn is_bool(&self) -> bool {
        match self {
            ValueType::Boolean(_) => true,
            // For schemas which don't tag booleans
            ValueType::Int32(Some(i)) if *i == 0 || *i == 1 => true,
            ValueType::Int64(Some(i)) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }
    /// `true` if the `Value` is an Array.
    pub fn is_array(&self) -> bool {
        matches!(self, ValueType::Array(_))
    }

    /// `true` if the `Value` is of UUID type.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn is_uuid(&self) -> bool {
        matches!(self, ValueType::Uuid(_))
    }

    /// `true` if the `Value` is a Date.
    pub fn is_date(&self) -> bool {
        matches!(self, ValueType::Date(_))
    }

    /// `true` if the `Value` is a DateTime.
    pub fn is_datetime(&self) -> bool {
        matches!(self, ValueType::DateTime(_))
    }

    /// `true` if the `Value` is a Time.
    pub fn is_time(&self) -> bool {
        matches!(self, ValueType::Time(_))
    }

    /// `true` if the `Value` is a JSON value.
    pub fn is_json(&self) -> bool {
        matches!(self, ValueType::Json(_))
    }

    /// Returns an `i64` if the value is a 64-bit signed integer, otherwise `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ValueType::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i32` if the value is a 32-bit signed integer, otherwise `None`.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            ValueType::Int32(i) => *i,
            _ => None,
        }
    }

    /// Returns an `i64` if the value is a signed integer, otherwise `None`.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            ValueType::Int32(i) => i.map(|i| i as i64),
            ValueType::Int64(i) => *i,
            _ => None,
        }
    }

    /// Returns a `f64` if the value is a double, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ValueType::Double(Some(f)) => Some(*f),
            _ => None,
        }
    }

    /// Returns a `f32` if the value is a double, otherwise `None`.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            ValueType::Float(Some(f)) => Some(*f),
            _ => None,
        }
    }
}

impl<'a> Value<'a> {
    /// Creates a new 32-bit signed integer.
    pub fn int32<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Value::from(ValueType::Int32(Some(value.into())))
    }

    /// Creates a new 64-bit signed integer.
    pub fn int64<I>(value: I) -> Self
    where
        I: Into<i64>,
    {
        Value::from(ValueType::Int64(Some(value.into())))
    }

    /// Creates a new 32-bit signed integer.
    pub fn integer<I>(value: I) -> Self
    where
        I: Into<i32>,
    {
        Value::from(ValueType::Int32(Some(value.into())))
    }

    /// Creates a new decimal value.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn numeric(value: BigDecimal) -> Self {
        Value::from(ValueType::Numeric(Some(value)))
    }

    /// Creates a new float value.
    pub fn float(value: f32) -> Self {
        Value::from(ValueType::Float(Some(value)))
    }

    /// Creates a new double value.
    pub fn double(value: f64) -> Self {
        Value::from(ValueType::Double(Some(value)))
    }

    /// Creates a new string value.
    pub fn text<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::from(ValueType::Text(Some(value.into())))
    }

    /// Creates a new enum value.
    pub fn enum_variant<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::from(ValueType::Enum(Some(EnumVariant::new(value)), None))
    }

    /// Creates a new enum value with the name of the enum attached.
    pub fn enum_variant_with_name<T, U, V>(value: T, name: U, schema_name: Option<V>) -> Self
    where
        T: Into<Cow<'a, str>>,
        U: Into<Cow<'a, str>>,
        V: Into<Cow<'a, str>>,
    {
        Value::from(ValueType::Enum(
            Some(EnumVariant::new(value)),
            Some(EnumName::new(name, schema_name)),
        ))
    }

    /// Creates a new bytes value.
    pub fn bytes<B>(value: B) -> Self
    where
        B: Into<Cow<'a, [u8]>>,
    {
        Value::from(ValueType::Bytes(Some(value.into())))
    }

    /// Creates a new boolean value.
    pub fn boolean<B>(value: B) -> Self
    where
        B: Into<bool>,
    {
        Value::from(ValueType::Boolean(Some(value.into())))
    }

    /// Creates a new character value.
    pub fn character<C>(value: C) -> Self
    where
        C: Into<char>,
    {
        Value::from(ValueType::Char(Some(value.into())))
    }

    /// Creates a new array value.
    pub fn array<I, V>(value: I) -> Self
    where
        I: IntoIterator<Item = V>,
        V: Into<Value<'a>>,
    {
        Value::from(ValueType::Array(Some(
            value.into_iter().map(|v| Value::from(v.into())).collect(),
        )))
    }

    /// Creates a new uuid value.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn uuid(value: Uuid) -> Self {
        Value::from(ValueType::Uuid(Some(value)))
    }

    /// Creates a new datetime value.
    pub fn datetime(value: DateTime<Utc>) -> Self {
        Value::from(ValueType::DateTime(Some(value)))
    }

    /// Creates a new date value.
    pub fn date(value: NaiveDate) -> Self {
        Value::from(ValueType::Date(Some(value)))
    }

    /// Creates a new time value.
    pub fn time(value: NaiveTime) -> Self {
        Value::from(ValueType::Time(Some(value)))
    }

    /// Creates a new JSON value.
    pub fn json(value: serde_json::Value) -> Self {
        Value::from(ValueType::Json(Some(value)))
    }

    /// Creates a new XML value.
    pub fn xml<T>(value: T) -> Self
    where
        T: Into<Cow<'a, str>>,
    {
        Value::from(ValueType::Xml(Some(value.into())))
    }

    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        self.typed.is_null()
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
        self.typed.is_text()
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match &self.typed {
            ValueType::Text(Some(cow)) => Some(cow.borrow()),
            ValueType::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub fn as_char(&self) -> Option<char> {
        match &self.typed {
            ValueType::Char(c) => *c,
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match &self.typed {
            ValueType::Text(Some(cow)) => Some(cow.to_string()),
            ValueType::Bytes(Some(cow)) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self.typed {
            ValueType::Text(Some(cow)) => Some(cow.into_owned()),
            ValueType::Bytes(Some(cow)) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        self.typed.is_bytes()
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match &self.typed {
            ValueType::Text(Some(cow)) => Some(cow.as_ref().as_bytes()),
            ValueType::Bytes(Some(cow)) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match &self.typed {
            ValueType::Text(Some(cow)) => Some(cow.to_string().into_bytes()),
            ValueType::Bytes(Some(cow)) => Some(cow.to_vec()),
            _ => None,
        }
    }

    /// Returns a bigdecimal, if the value is a numeric, float or double value,
    /// otherwise `None`.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn into_numeric(self) -> Option<BigDecimal> {
        match self.typed {
            ValueType::Numeric(d) => d,
            ValueType::Float(f) => f.and_then(BigDecimal::from_f32),
            ValueType::Double(f) => f.and_then(BigDecimal::from_f64),
            _ => None,
        }
    }

    /// Returns a reference to a bigdecimal, if the value is a numeric.
    /// Otherwise `None`.
    #[cfg(feature = "bigdecimal")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "bigdecimal")))]
    pub fn as_numeric(&self) -> Option<&BigDecimal> {
        match &self.typed {
            ValueType::Numeric(d) => d.as_ref(),
            _ => None,
        }
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match &self.typed {
            ValueType::Boolean(b) => *b,
            // For schemas which don't tag booleans
            ValueType::Int32(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            ValueType::Int64(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid")]
    #[cfg_attr(feature = "docs", doc(cfg(feature = "uuid")))]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match &self.typed {
            ValueType::Uuid(u) => *u,
            _ => None,
        }
    }

    /// `true` if the `Value` is a DateTime.
    pub fn is_datetime(&self) -> bool {
        self.typed.is_datetime()
    }

    /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match &self.typed {
            ValueType::DateTime(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a Date.
    pub fn is_date(&self) -> bool {
        self.typed.is_date()
    }

    /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
    pub fn as_date(&self) -> Option<NaiveDate> {
        match &self.typed {
            ValueType::Date(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a `Time`.
    pub fn is_time(&self) -> bool {
        self.typed.is_time()
    }

    /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
    pub fn as_time(&self) -> Option<NaiveTime> {
        match &self.typed {
            ValueType::Time(time) => *time,
            _ => None,
        }
    }

    /// `true` if the `Value` is a JSON value.
    pub fn is_json(&self) -> bool {
        self.typed.is_json()
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match &self.typed {
            ValueType::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    pub fn into_json(self) -> Option<serde_json::Value> {
        match self.typed {
            ValueType::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Returns a `Vec<T>` if the value is an array of `T`, otherwise `None`.
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<Value>
        T: TryFrom<ValueType<'a>>,
    {
        match self.typed {
            ValueType::Array(Some(vec)) => {
                let rslt: Result<Vec<_>, _> = vec.into_iter().map(|val| val.typed.try_into()).collect();
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
        match &self.typed {
            ValueType::Array(Some(vec)) => {
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
            .typed
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for i32 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i32, Self::Error> {
        value
            .typed
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
            .typed
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
        match &value.typed {
            ValueType::Text(Some(_)) => {
                let text = value.as_str().unwrap();

                match std::net::IpAddr::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            ValueType::Bytes(Some(_)) => {
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
        match &value.typed {
            ValueType::Uuid(uuid) => Ok(*uuid),
            ValueType::Text(Some(_)) => {
                let text = value.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            ValueType::Bytes(Some(_)) => {
                let text = value.as_str().unwrap();

                match uuid::Uuid::from_str(text) {
                    Ok(ip) => Ok(Some(ip)),
                    Err(e) => Err(e.into()),
                }
            }
            _ if value.is_null() => Ok(None),
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
        let pv = Value::from(ValueType::array(vec![1]));
        let values: Vec<i32> = pv.into_vec().expect("convert into Vec<i32>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_ints64_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueType::array(vec![1_i64]));
        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");
        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueType::array(vec![1.0]));
        let values: Vec<f64> = pv.into_vec().expect("convert into Vec<f64>");
        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueType::array(vec!["test"]));
        let values: Vec<String> = pv.into_vec().expect("convert into Vec<String>");
        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = Value::from(ValueType::array(vec![true]));
        let values: Vec<bool> = pv.into_vec().expect("convert into Vec<bool>");
        assert_eq!(values, vec![true]);
    }

    #[test]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = Value::from(ValueType::array(vec![datetime]));
        let values: Vec<DateTime<Utc>> = pv.into_vec().expect("convert into Vec<DateTime>");
        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = Value::from(ValueType::array(vec![1]));
        let rslt: Option<Vec<f64>> = pv.into_vec();
        assert!(rslt.is_none());
    }

    #[test]
    fn display_format_for_datetime() {
        let dt: DateTime<Utc> = DateTime::from_str("2019-07-27T05:30:30Z").expect("failed while parsing date");
        let pv = Value::from(ValueType::datetime(dt));

        assert_eq!(format!("{pv}"), "\"2019-07-27 05:30:30 UTC\"");
    }

    #[test]
    fn display_format_for_date() {
        let date = NaiveDate::from_ymd_opt(2022, 8, 11).unwrap();
        let pv = Value::from(ValueType::date(date));

        assert_eq!(format!("{pv}"), "\"2022-08-11\"");
    }

    #[test]
    fn display_format_for_time() {
        let time = NaiveTime::from_hms_opt(16, 17, 00).unwrap();
        let pv = Value::from(ValueType::time(time));

        assert_eq!(format!("{pv}"), "\"16:17:00\"");
    }

    #[test]
    #[cfg(feature = "uuid")]
    fn display_format_for_uuid() {
        let id = Uuid::from_str("67e5504410b1426f9247bb680e5fe0c8").unwrap();
        let pv = Value::from(ValueType::uuid(id));

        assert_eq!(format!("{pv}"), "\"67e55044-10b1-426f-9247-bb680e5fe0c8\"");
    }
}

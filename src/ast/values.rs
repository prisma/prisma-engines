use crate::ast::*;
use crate::error::{Error, ErrorKind};
use rust_decimal::{
    prelude::{FromPrimitive, ToPrimitive},
    Decimal,
};
use std::{
    borrow::{Borrow, Cow},
    convert::TryFrom,
    fmt,
    str::FromStr,
};

#[cfg(feature = "json-1")]
use serde_json::{Number, Value as JsonValue};

#[cfg(feature = "uuid-0_8")]
use uuid::Uuid;

#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};

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
    /// 64-bit signed integer.
    Integer(Option<i64>),
    /// A decimal value.
    Real(Option<Decimal>),
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
    #[cfg(feature = "json-1")]
    /// A JSON value.
    Json(Option<serde_json::Value>),
    /// A XML value.
    Xml(Option<Cow<'a, str>>),
    #[cfg(feature = "uuid-0_8")]
    /// An UUID value.
    Uuid(Option<Uuid>),
    #[cfg(feature = "chrono-0_4")]
    /// A datetime value.
    DateTime(Option<DateTime<Utc>>),
    #[cfg(feature = "chrono-0_4")]
    /// A date value.
    Date(Option<NaiveDate>),
    #[cfg(feature = "chrono-0_4")]
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
            Value::Integer(val) => val.map(|v| write!(f, "{}", v)),
            Value::Real(val) => val.map(|v| write!(f, "{}", v)),
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
            #[cfg(feature = "json-1")]
            Value::Json(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            Value::Xml(val) => val.as_ref().map(|v| write!(f, "{}", v)),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(val) => val.map(|v| write!(f, "{}", v)),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(val) => val.map(|v| write!(f, "{}", v)),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(val) => val.map(|v| write!(f, "{}", v)),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(val) => val.map(|v| write!(f, "{}", v)),
        };

        match res {
            Some(r) => r,
            None => write!(f, "null"),
        }
    }
}

#[cfg(feature = "json-1")]
impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        let res = match pv {
            Value::Integer(i) => i.map(|i| serde_json::Value::Number(Number::from(i))),
            Value::Real(d) => d.map(|d| serde_json::to_value(d).unwrap()),
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
            #[cfg(feature = "json-1")]
            Value::Json(v) => v,
            Value::Xml(cow) => cow.map(|cow| serde_json::Value::String(cow.into_owned())),
            Value::Array(v) => {
                v.map(|v| serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect()))
            }
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(u) => u.map(|u| serde_json::Value::String(u.to_hyphenated().to_string())),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(dt) => dt.map(|dt| serde_json::Value::String(dt.to_rfc3339())),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(date) => date.map(|date| serde_json::Value::String(format!("{}", date))),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(time) => time.map(|time| serde_json::Value::String(format!("{}", time))),
        };

        match res {
            Some(val) => val,
            None => serde_json::Value::Null,
        }
    }
}

impl<'a> Value<'a> {
    /// Creates a new integer value.
    pub fn integer<I>(value: I) -> Self
    where
        I: Into<i64>,
    {
        Value::Integer(Some(value.into()))
    }

    /// Creates a new decimal value.
    pub fn real(value: Decimal) -> Self {
        Value::Real(Some(value))
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
    #[cfg(feature = "uuid-0_8")]
    pub fn uuid(value: Uuid) -> Self {
        Value::Uuid(Some(value))
    }

    /// Creates a new datetime value.
    #[cfg(feature = "chrono-0_4")]
    pub fn datetime(value: DateTime<Utc>) -> Self {
        Value::DateTime(Some(value))
    }

    /// Creates a new date value.
    #[cfg(feature = "chrono-0_4")]
    pub fn date(value: NaiveDate) -> Self {
        Value::Date(Some(value))
    }

    /// Creates a new time value.
    #[cfg(feature = "chrono-0_4")]
    pub fn time(value: NaiveTime) -> Self {
        Value::Time(Some(value))
    }

    /// Creates a new JSON value.
    #[cfg(feature = "json-1")]
    pub fn json(value: serde_json::Value) -> Self {
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
    pub fn is_null(&self) -> bool {
        match self {
            Value::Integer(i) => i.is_none(),
            Value::Real(r) => r.is_none(),
            Value::Text(t) => t.is_none(),
            Value::Enum(e) => e.is_none(),
            Value::Bytes(b) => b.is_none(),
            Value::Boolean(b) => b.is_none(),
            Value::Char(c) => c.is_none(),
            Value::Array(v) => v.is_none(),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(u) => u.is_none(),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(dt) => dt.is_none(),
            #[cfg(feature = "chrono-0_4")]
            Value::Date(d) => d.is_none(),
            #[cfg(feature = "chrono-0_4")]
            Value::Time(t) => t.is_none(),
            #[cfg(feature = "json-1")]
            Value::Json(json) => json.is_none(),
            Value::Xml(s) => s.is_none(),
        }
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
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
    pub fn as_char(&self) -> Option<char> {
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
    pub fn is_bytes(&self) -> bool {
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

    /// `true` if the `Value` is an integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Returns an i64 if the value is an integer, otherwise `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => *i,
            _ => None,
        }
    }

    /// `true` if the `Value` is a real value.
    pub fn is_real(&self) -> bool {
        matches!(self, Value::Real(_))
    }

    /// Returns a f64 if the value is a real value and the underlying decimal can be converted, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Real(Some(d)) => d.to_f64(),
            _ => None,
        }
    }

    /// Returns a decimal if the value is a real value, otherwise `None`.
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Value::Real(d) => *d,
            _ => None,
        }
    }

    /// `true` if the `Value` is a boolean value.
    pub fn is_bool(&self) -> bool {
        match self {
            Value::Boolean(_) => true,
            // For schemas which don't tag booleans
            Value::Integer(Some(i)) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => *b,
            // For schemas which don't tag booleans
            Value::Integer(Some(i)) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// `true` if the `Value` is an Array.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// `true` if the `Value` is of UUID type.
    #[cfg(feature = "uuid-0_8")]
    pub fn is_uuid(&self) -> bool {
        matches!(self, Value::Uuid(_))
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid-0_8")]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Value::Uuid(u) => *u,
            _ => None,
        }
    }

    /// `true` if the `Value` is a DateTime.
    #[cfg(feature = "chrono-0_4")]
    pub fn is_datetime(&self) -> bool {
        matches!(self, Value::DateTime(_))
    }

    /// Returns a `DateTime` if the value is a `DateTime`, otherwise `None`.
    #[cfg(feature = "chrono-0_4")]
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match self {
            Value::DateTime(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a Date.
    #[cfg(feature = "chrono-0_4")]
    pub fn is_date(&self) -> bool {
        matches!(self, Value::Date(_))
    }

    /// Returns a `NaiveDate` if the value is a `Date`, otherwise `None`.
    #[cfg(feature = "chrono-0_4")]
    pub fn as_date(&self) -> Option<NaiveDate> {
        match self {
            Value::Date(dt) => *dt,
            _ => None,
        }
    }

    /// `true` if the `Value` is a `Time`.
    #[cfg(feature = "chrono-0_4")]
    pub fn is_time(&self) -> bool {
        matches!(self, Value::Time(_))
    }

    /// Returns a `NaiveTime` if the value is a `Time`, otherwise `None`.
    #[cfg(feature = "chrono-0_4")]
    pub fn as_time(&self) -> Option<NaiveTime> {
        match self {
            Value::Time(time) => *time,
            _ => None,
        }
    }

    /// `true` if the `Value` is a JSON value.
    #[cfg(feature = "json-1")]
    pub fn is_json(&self) -> bool {
        matches!(self, Value::Json(_))
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json-1")]
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Value::Json(Some(j)) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json-1")]
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

value!(val: i64, Integer, val);
value!(val: bool, Boolean, val);
value!(val: Decimal, Real, val);
#[cfg(feature = "json-1")]
value!(val: JsonValue, Json, val);
#[cfg(feature = "uuid-0_8")]
value!(val: Uuid, Uuid, val);
value!(val: &'a str, Text, val.into());
value!(val: String, Text, val.into());
value!(val: usize, Integer, i64::try_from(val).unwrap());
value!(val: i32, Integer, i64::try_from(val).unwrap());
value!(val: &'a [u8], Bytes, val.into());
#[cfg(feature = "chrono-0_4")]
value!(val: DateTime<Utc>, DateTime, val);
#[cfg(feature = "chrono-0_4")]
value!(val: chrono::NaiveTime, Time, val);
#[cfg(feature = "chrono-0_4")]
value!(val: chrono::NaiveDate, Date, val);

value!(
    val: f64,
    Real,
    Decimal::from_str(&val.to_string()).expect("f64 is not a Decimal")
);

value!(val: f32, Real, Decimal::from_f32(val).expect("f32 is not a Decimal"));

impl<'a> TryFrom<Value<'a>> for i64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i64, Self::Error> {
        value
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not an i64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for Decimal {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<Decimal, Self::Error> {
        value
            .as_decimal()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a decimal")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for f64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<f64, Self::Error> {
        value
            .as_decimal()
            .and_then(|d| d.to_f64())
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

#[cfg(feature = "chrono-0_4")]
impl<'a> TryFrom<Value<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or_else(|| Error::builder(ErrorKind::conversion("Not a datetime")).build())
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

#[cfg(all(test, feature = "postgresql"))]
mod tests {
    use super::*;
    #[cfg(feature = "chrono-0_4")]
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints_can_be_converted_into_a_vec() {
        let pv = Value::array(vec![1]);
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
    #[cfg(feature = "chrono-0_4")]
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
}

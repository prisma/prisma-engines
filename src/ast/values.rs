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
use chrono::{DateTime, Utc};

/// A value we must parameterize for the prepared statement.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Null,
    Integer(i64),
    Real(Decimal),
    Text(Cow<'a, str>),
    Enum(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Boolean(bool),
    Char(char),
    #[cfg(all(feature = "array", feature = "postgresql"))]
    Array(Vec<Value<'a>>),
    #[cfg(feature = "json-1")]
    Json(serde_json::Value),
    #[cfg(feature = "uuid-0_8")]
    Uuid(Uuid),
    #[cfg(feature = "chrono-0_4")]
    DateTime(DateTime<Utc>),
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
        match self {
            Value::Null => write!(f, "null"),
            Value::Integer(val) => write!(f, "{}", val),
            Value::Real(val) => write!(f, "{}", val),
            Value::Text(val) => write!(f, "\"{}\"", val),
            Value::Bytes(val) => write!(f, "<{} bytes blob>", val.len()),
            Value::Enum(val) => write!(f, "\"{}\"", val),
            Value::Boolean(val) => write!(f, "{}", val),
            Value::Char(val) => write!(f, "'{}'", val),
            #[cfg(feature = "array")]
            Value::Array(vals) => {
                let len = vals.len();

                write!(f, "[")?;
                for (i, val) in vals.iter().enumerate() {
                    write!(f, "{}", val)?;

                    if i < (len - 1) {
                        write!(f, ",")?;
                    }
                }
                write!(f, "]")
            }
            #[cfg(feature = "json-1")]
            Value::Json(val) => write!(f, "{}", val),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(val) => write!(f, "{}", val),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(val) => write!(f, "{}", val),
        }
    }
}

#[cfg(feature = "json-1")]
impl<'a> From<Value<'a>> for serde_json::Value {
    fn from(pv: Value<'a>) -> Self {
        match pv {
            Value::Null => serde_json::Value::Null,
            Value::Integer(i) => serde_json::Value::Number(Number::from(i)),
            Value::Real(d) => serde_json::to_value(d).unwrap(),
            Value::Text(cow) => serde_json::Value::String(cow.into_owned()),
            Value::Bytes(bytes) => serde_json::Value::String(base64::encode(&bytes)),
            Value::Enum(cow) => serde_json::Value::String(cow.into_owned()),
            Value::Boolean(b) => serde_json::Value::Bool(b),
            Value::Char(c) => {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                serde_json::Value::String(s)
            }
            Value::Json(v) => v,
            #[cfg(feature = "array")]
            Value::Array(v) => serde_json::Value::Array(v.into_iter().map(serde_json::Value::from).collect()),
            #[cfg(feature = "uuid-0_8")]
            Value::Uuid(u) => serde_json::Value::String(u.to_hyphenated().to_string()),
            #[cfg(feature = "chrono-0_4")]
            Value::DateTime(dt) => serde_json::Value::String(dt.to_rfc3339()),
        }
    }
}

impl<'a> Value<'a> {
    /// `true` if the `Value` is null.
    pub fn is_null(&self) -> bool {
        match self {
            Value::Null => true,
            _ => false,
        }
    }

    /// `true` if the `Value` is text.
    pub fn is_text(&self) -> bool {
        match self {
            Value::Text(_) => true,
            _ => false,
        }
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Text(cow) => Some(cow.borrow()),
            Value::Bytes(cow) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub fn as_char(&self) -> Option<char> {
        match self {
            Value::Char(c) => Some(*c),
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match self {
            Value::Text(cow) => Some(cow.to_string()),
            Value::Bytes(cow) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `Value` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self {
            Value::Text(cow) => Some(cow.into_owned()),
            Value::Bytes(cow) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        match self {
            Value::Bytes(_) => true,
            _ => false,
        }
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Text(cow) => Some(cow.as_ref().as_bytes()),
            Value::Bytes(cow) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Value::Text(cow) => Some(cow.to_string().into_bytes()),
            Value::Bytes(cow) => Some(cow.to_owned().into()),
            _ => None,
        }
    }

    /// `true` if the `Value` is an integer.
    pub fn is_integer(&self) -> bool {
        match self {
            Value::Integer(_) => true,
            _ => false,
        }
    }

    /// Returns an i64 if the value is an integer, otherwise `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// `true` if the `Value` is a real value.
    pub fn is_real(&self) -> bool {
        match self {
            Value::Real(_) => true,
            _ => false,
        }
    }

    /// Returns a f64 if the value is a real value and the underlying decimal can be converted, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Real(d) => d.to_f64(),
            _ => None,
        }
    }

    /// Returns a decimal if the value is a real value, otherwise `None`.
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Value::Real(d) => Some(*d),
            _ => None,
        }
    }

    /// `true` if the `Value` is a boolean value.
    pub fn is_bool(&self) -> bool {
        match self {
            Value::Boolean(_) => true,
            // For schemas which don't tag booleans
            Value::Integer(i) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Boolean(b) => Some(*b),
            // For schemas which don't tag booleans
            Value::Integer(i) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// `true` if the `Value` is of UUID type.
    #[cfg(feature = "uuid-0_8")]
    pub fn is_uuid(&self) -> bool {
        match self {
            Value::Uuid(_) => true,
            _ => false,
        }
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid-0_8")]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            Value::Uuid(u) => Some(*u),
            _ => None,
        }
    }

    /// `true` if the `Value` is a DateTime.
    #[cfg(feature = "chrono-0_4")]
    pub fn is_datetime(&self) -> bool {
        match self {
            Value::DateTime(_) => true,
            _ => false,
        }
    }

    /// Returns a DateTime if the value is a DateTime, otherwise `None`.
    #[cfg(feature = "chrono-0_4")]
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match self {
            Value::DateTime(dt) => Some(*dt),
            _ => None,
        }
    }

    /// `true` if the `Value` is a JSON value.
    #[cfg(feature = "json-1")]
    pub fn is_json(&self) -> bool {
        match self {
            Value::Json(_) => true,
            _ => false,
        }
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json-1")]
    pub fn as_json(&self) -> Option<&serde_json::Value> {
        match self {
            Value::Json(j) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json-1")]
    pub fn into_json(self) -> Option<serde_json::Value> {
        match self {
            Value::Json(j) => Some(j),
            _ => None,
        }
    }

    /// Returns a Vec<T> if the value is an array of T, otherwise `None`.
    #[cfg(all(feature = "array", feature = "postgresql"))]
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<Value>
        T: TryFrom<Value<'a>>,
    {
        match self {
            Value::Array(vec) => {
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

impl<'a> From<&'a str> for Value<'a> {
    fn from(that: &'a str) -> Self {
        Value::Text(that.into())
    }
}

impl<'a> From<String> for Value<'a> {
    fn from(that: String) -> Self {
        Value::Text(that.into())
    }
}

impl<'a> From<usize> for Value<'a> {
    fn from(that: usize) -> Self {
        Value::Integer(i64::try_from(that).unwrap())
    }
}

impl<'a> From<i32> for Value<'a> {
    fn from(that: i32) -> Self {
        Value::Integer(i64::try_from(that).unwrap())
    }
}

impl<'a> From<&'a [u8]> for Value<'a> {
    fn from(that: &'a [u8]) -> Value<'a> {
        Value::Bytes(that.into())
    }
}

impl<'a, T> From<Option<T>> for Value<'a>
where
    T: Into<Value<'a>>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(value) => value.into(),
            None => Value::Null,
        }
    }
}

impl<'a> TryFrom<Value<'a>> for i64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<i64, Self::Error> {
        value
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not an i64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for Decimal {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<Decimal, Self::Error> {
        value
            .as_decimal()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a decimal")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for f64 {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<f64, Self::Error> {
        value
            .as_decimal()
            .and_then(|d| d.to_f64())
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a f64")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for String {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<String, Self::Error> {
        value
            .into_string()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a string")).build())
    }
}

impl<'a> TryFrom<Value<'a>> for bool {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<bool, Self::Error> {
        value
            .as_bool()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a bool")).build())
    }
}

#[cfg(feature = "chrono-0_4")]
impl<'a> TryFrom<Value<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: Value<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a datetime")).build())
    }
}

macro_rules! value {
    ($kind:ident,$paramkind:ident) => {
        impl<'a> From<$kind> for Value<'a> {
            fn from(that: $kind) -> Self {
                Value::$paramkind(that)
            }
        }
    };
}

value!(i64, Integer);
value!(bool, Boolean);
value!(Decimal, Real);

#[cfg(feature = "json-1")]
value!(JsonValue, Json);

#[cfg(feature = "uuid-0_8")]
value!(Uuid, Uuid);

#[cfg(feature = "chrono-0_4")]
impl<'a> From<DateTime<Utc>> for Value<'a> {
    fn from(that: DateTime<Utc>) -> Self {
        Value::DateTime(that)
    }
}

#[cfg(feature = "chrono-0_4")]
impl<'a> From<chrono::NaiveTime> for Value<'a> {
    fn from(that: chrono::NaiveTime) -> Self {
        Value::Text(that.to_string().into())
    }
}

impl<'a> From<f64> for Value<'a> {
    fn from(that: f64) -> Self {
        // Decimal::from_f64 is buggy. See https://github.com/paupino/rust-decimal/issues/228
        let dec = Decimal::from_str(&that.to_string()).expect("f64 is not a Decimal");
        Value::Real(dec)
    }
}

impl<'a> From<f32> for Value<'a> {
    fn from(that: f32) -> Self {
        let dec = Decimal::from_f32(that).expect("f32 is not a Decimal");
        Value::Real(dec)
    }
}

/*
 * Here be the database value converters.
 */

#[cfg(all(test, feature = "array", feature = "postgresql"))]
mod tests {
    use super::*;
    #[cfg(feature = "chrono-0_4")]
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints_can_be_converted_into_a_vec() {
        let pv = Value::Array(vec![Value::Integer(1)]);

        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");

        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = Value::Array(vec![Value::from(1.0)]);

        let values: Vec<f64> = pv.into_vec().expect("convert into Vec<f64>");

        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = Value::Array(vec![Value::Text(Cow::from("test"))]);

        let values: Vec<String> = pv.into_vec().expect("convert into Vec<String>");

        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = Value::Array(vec![Value::Boolean(true)]);

        let values: Vec<bool> = pv.into_vec().expect("convert into Vec<bool>");

        assert_eq!(values, vec![true]);
    }

    #[test]
    #[cfg(feature = "chrono-0_4")]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = Value::Array(vec![Value::DateTime(datetime)]);

        let values: Vec<DateTime<Utc>> = pv.into_vec().expect("convert into Vec<DateTime>");

        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = Value::Array(vec![Value::Integer(1)]);

        let rslt: Option<Vec<f64>> = pv.into_vec();

        assert!(rslt.is_none());
    }
}

/// An in-memory temporary table. Can be used in some of the databases in a
/// place of an actual table. Doesn't work in MySQL 5.7.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Values<'a> {
    pub(crate) rows: Vec<Row<'a>>,
}

impl<'a> Values<'a> {
    /// Create a new in-memory set of values.
    pub fn new() -> Self {
        Self::default()
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

#[macro_export]
macro_rules! values {
    ($($x:expr),*) => (
        Values::from(std::iter::empty() $(.chain(std::iter::once(Row::from($x))))*)
    );
}

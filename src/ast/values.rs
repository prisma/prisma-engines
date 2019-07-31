use crate::ast::*;
use crate::error::Error;
use std::{
    borrow::{Borrow, Cow},
    convert::TryFrom,
    fmt,
};

#[cfg(feature = "json-1")]
use serde_json::{Number, Value};

#[cfg(feature = "uuid-0_7")]
use uuid::Uuid;

#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Utc};

/// A value we must parameterize for the prepared statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedValue<'a> {
    Null,
    Integer(i64),
    Real(f64),
    Text(Cow<'a, str>),
    Boolean(bool),
    #[cfg(feature = "array")]
    Array(Vec<ParameterizedValue<'a>>),
    #[cfg(feature = "json-1")]
    Json(Value),
    #[cfg(feature = "uuid-0_7")]
    Uuid(Uuid),
    #[cfg(feature = "chrono-0_4")]
    DateTime(DateTime<Utc>),
}

pub(crate) struct Params<'a>(pub(crate) &'a [ParameterizedValue<'a>]);

impl<'a> fmt::Display for Params<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for val in self.0.iter() {
            write!(f, "{},", val)?;
        }
        write!(f, "]")
    }
}

impl<'a> fmt::Display for ParameterizedValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterizedValue::Null => write!(f, "null"),
            ParameterizedValue::Integer(val) => write!(f, "{}", val),
            ParameterizedValue::Real(val) => write!(f, "{}", val),
            ParameterizedValue::Text(val) => write!(f, "{}", val),
            ParameterizedValue::Boolean(val) => write!(f, "{}", val),
            #[cfg(feature = "array")]
            ParameterizedValue::Array(vals) => {
                write!(f, "[")?;

                for val in vals.iter() {
                    write!(f, "{},", val)?;
                }

                write!(f, "]")
            }
            #[cfg(feature = "json-1")]
            ParameterizedValue::Json(val) => write!(f, "{}", val),
            #[cfg(feature = "uuid-0_7")]
            ParameterizedValue::Uuid(val) => write!(f, "{}", val),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(val) => write!(f, "{}", val),
        }
    }
}

#[cfg(feature = "json-1")]
impl<'a> From<ParameterizedValue<'a>> for Value {
    fn from(pv: ParameterizedValue<'a>) -> Self {
        match pv {
            ParameterizedValue::Null => Value::Null,
            ParameterizedValue::Integer(i) => Value::Number(Number::from(i)),
            ParameterizedValue::Real(f) => Value::Number(Number::from_f64(f).unwrap()),
            ParameterizedValue::Text(cow) => Value::String(cow.into_owned()),
            ParameterizedValue::Boolean(b) => Value::Bool(b),
            ParameterizedValue::Json(v) => v,
            #[cfg(feature = "array")]
            ParameterizedValue::Array(v) => Value::Array(v.into_iter().map(Value::from).collect()),
            #[cfg(feature = "uuid-0_7")]
            ParameterizedValue::Uuid(u) => Value::String(u.to_hyphenated().to_string()),
            #[cfg(feature = "chrono-0_4")]
            ParameterizedValue::DateTime(dt) => Value::String(dt.to_rfc3339()),
        }
    }
}

impl<'a> ParameterizedValue<'a> {
    /// `true` if the `ParameterizedValue` is null.
    pub fn is_null(&self) -> bool {
        match self {
            ParameterizedValue::Null => true,
            _ => false,
        }
    }

    /// `true` if the `ParameterizedValue` is text.
    pub fn is_text(&self) -> bool {
        match self {
            ParameterizedValue::Text(_) => true,
            _ => false,
        }
    }

    /// Returns a &str if the value is text, otherwise `None`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.borrow()),
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.to_string()),
            _ => None,
        }
    }

    /// Transforms the `ParameterizedValue` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.into_owned()),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is an integer.
    pub fn is_integer(&self) -> bool {
        match self {
            ParameterizedValue::Integer(_) => true,
            _ => false,
        }
    }

    /// Returns an i64 if the value is an integer, otherwise `None`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ParameterizedValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is a real value.
    pub fn is_real(&self) -> bool {
        match self {
            ParameterizedValue::Real(_) => true,
            _ => false,
        }
    }

    /// Returns a f64 if the value is a real value, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParameterizedValue::Real(f) => Some(*f),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is a boolean value.
    pub fn is_bool(&self) -> bool {
        match self {
            ParameterizedValue::Boolean(_) => true,
            // For schemas which don't tag booleans
            ParameterizedValue::Integer(i) if *i == 0 || *i == 1 => true,
            _ => false,
        }
    }

    /// Returns a bool if the value is a boolean, otherwise `None`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ParameterizedValue::Boolean(b) => Some(*b),
            // For schemas which don't tag booleans
            ParameterizedValue::Integer(i) if *i == 0 || *i == 1 => Some(*i == 1),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is of UUID type.
    #[cfg(feature = "uuid-0_7")]
    pub fn is_uuid(&self) -> bool {
        match self {
            ParameterizedValue::Uuid(_) => true,
            _ => false,
        }
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid-0_7")]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            ParameterizedValue::Uuid(u) => Some(*u),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is a DateTime.
    #[cfg(feature = "uuid-0_7")]
    pub fn is_datetime(&self) -> bool {
        match self {
            ParameterizedValue::DateTime(_) => true,
            _ => false,
        }
    }

    /// Returns a DateTime if the value is a DateTime, otherwise `None`.
    #[cfg(feature = "chrono-0_4")]
    pub fn as_datetime(&self) -> Option<DateTime<Utc>> {
        match self {
            ParameterizedValue::DateTime(dt) => Some(*dt),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is a JSON value.
    #[cfg(feature = "json-1")]
    pub fn is_json(&self) -> bool {
        match self {
            ParameterizedValue::Json(_) => true,
            _ => false,
        }
    }

    /// Returns a reference to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json-1")]
    pub fn as_json(&self) -> Option<&Value> {
        match self {
            ParameterizedValue::Json(j) => Some(j),
            _ => None,
        }
    }

    /// Transforms to a JSON Value if of Json type, otherwise `None`.
    #[cfg(feature = "json-1")]
    pub fn into_json(self) -> Option<Value> {
        match self {
            ParameterizedValue::Json(j) => Some(j),
            _ => None,
        }
    }

    /// Returns a Vec<T> if the value is an array of T, otherwise `None`.
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<ParameterizedValue>
        T: TryFrom<ParameterizedValue<'a>>,
    {
        match self {
            ParameterizedValue::Array(vec) => {
                let rslt: Result<Vec<_>, _> = vec.into_iter().map(|pv| T::try_from(pv)).collect();
                match rslt {
                    Err(_) => None,
                    Ok(values) => Some(values),
                }
            }
            _ => None,
        }
    }
}

/// A value we can compare and use in database queries.
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseValue<'a> {
    /// Anything that we must parameterize before querying
    Parameterized(ParameterizedValue<'a>),
    /// A database column
    Column(Box<Column<'a>>),
    /// Data in a row form, e.g. (1, 2, 3)
    Row(Row<'a>),
    /// A nested `SELECT` statement
    Select(Select<'a>),
    /// A database function call
    Function(Function<'a>),
    /// A qualified asterisk to a table
    Asterisk(Option<Table<'a>>),
}

/// A quick alias to create an asterisk to a table.
///
/// ```rust
/// # use prisma_query::ast::*;
/// assert_eq!(
///     asterisk(),
///     DatabaseValue::Asterisk(None)
/// )
/// ```
#[inline]
pub fn asterisk() -> DatabaseValue<'static> {
    DatabaseValue::Asterisk(None)
}

/*
 * Here be the parameterized value converters.
 */

impl<'a> From<&'a str> for ParameterizedValue<'a> {
    fn from(that: &'a str) -> Self {
        ParameterizedValue::Text(that.into())
    }
}

impl<'a> From<String> for ParameterizedValue<'a> {
    fn from(that: String) -> Self {
        ParameterizedValue::Text(that.into())
    }
}

impl<'a> From<usize> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: usize) -> Self {
        ParameterizedValue::Integer(that as i64)
    }
}

impl<'a> From<i32> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: i32) -> Self {
        ParameterizedValue::Integer(that as i64)
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for i64 {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<i64, Self::Error> {
        value.as_i64().ok_or(Error::ConversionError("Not an i64"))
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for f64 {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<f64, Self::Error> {
        value.as_f64().ok_or(Error::ConversionError("Not an f64"))
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for String {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<String, Self::Error> {
        value
            .into_string()
            .ok_or(Error::ConversionError("Not a string"))
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for bool {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<bool, Self::Error> {
        value.as_bool().ok_or(Error::ConversionError("Not a bool"))
    }
}

#[cfg(feature = "chrono-0_4")]
impl<'a> TryFrom<ParameterizedValue<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or(Error::ConversionError("Not a datetime"))
    }
}

macro_rules! parameterized_value {
    ($kind:ident,$paramkind:ident) => {
        impl<'a> From<$kind> for ParameterizedValue<'a> {
            fn from(that: $kind) -> Self {
                ParameterizedValue::$paramkind(that)
            }
        }
    };
}

parameterized_value!(i64, Integer);
parameterized_value!(f64, Real);
parameterized_value!(bool, Boolean);

#[cfg(feature = "json-1")]
parameterized_value!(Value, Json);

#[cfg(feature = "uuid-0_7")]
parameterized_value!(Uuid, Uuid);

#[cfg(feature = "chrono-0_4")]
impl<'a> From<DateTime<Utc>> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: DateTime<Utc>) -> Self {
        ParameterizedValue::DateTime(that)
    }
}

/*
 * Here be the database value converters.
 */

macro_rules! database_value {
    ($kind:ident,$paramkind:ident) => {
        impl<'a> From<$kind<'a>> for DatabaseValue<'a> {
            fn from(that: $kind<'a>) -> Self {
                DatabaseValue::$paramkind(that)
            }
        }
    };
}

database_value!(Row, Row);
database_value!(Function, Function);

impl<'a, T> From<T> for DatabaseValue<'a>
where
    T: Into<ParameterizedValue<'a>>,
{
    #[inline]
    fn from(p: T) -> Self {
        DatabaseValue::Parameterized(p.into())
    }
}

impl<'a, T> From<Vec<T>> for DatabaseValue<'a>
where
    T: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from(v: Vec<T>) -> Self {
        let row: Row<'a> = v.into();
        row.into()
    }
}

impl<'a> Comparable<'a> for DatabaseValue<'a> {
    #[inline]
    fn equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::Equals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn not_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::NotEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn less_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::LessThan(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn less_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::LessThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn greater_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::GreaterThan(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::GreaterThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::In(Box::new(self), Box::new(selection.into()))
    }

    #[inline]
    fn not_in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        Compare::NotIn(Box::new(self), Box::new(selection.into()))
    }

    #[inline]
    fn like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::Like(Box::new(self), pattern.into())
    }

    #[inline]
    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::NotLike(Box::new(self), pattern.into())
    }

    #[inline]
    fn begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::BeginsWith(Box::new(self), pattern.into())
    }

    #[inline]
    fn not_begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::NotBeginsWith(Box::new(self), pattern.into())
    }

    #[inline]
    fn ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::EndsInto(Box::new(self), pattern.into())
    }

    #[inline]
    fn not_ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        Compare::NotEndsInto(Box::new(self), pattern.into())
    }

    #[inline]
    fn is_null(self) -> Compare<'a> {
        Compare::Null(Box::new(self))
    }

    #[inline]
    fn is_not_null(self) -> Compare<'a> {
        Compare::NotNull(Box::new(self))
    }

    #[inline]
    fn between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
        V: Into<DatabaseValue<'a>>,
    {
        Compare::Between(
            Box::new(self),
            Box::new(left.into()),
            Box::new(right.into()),
        )
    }

    #[inline]
    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
        V: Into<DatabaseValue<'a>>,
    {
        Compare::NotBetween(
            Box::new(self),
            Box::new(left.into()),
            Box::new(right.into()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints_can_be_converted_into_a_vec() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::Integer(1)]);

        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");

        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::Real(1.0)]);

        let values: Vec<f64> = pv.into_vec().expect("convert into Vec<f64>");

        assert_eq!(values, vec![1.0]);
    }

    #[test]
    fn a_parameterized_value_of_texts_can_be_converted_into_a_vec() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::Text(Cow::from("test"))]);

        let values: Vec<String> = pv.into_vec().expect("convert into Vec<String>");

        assert_eq!(values, vec!["test"]);
    }

    #[test]
    fn a_parameterized_value_of_booleans_can_be_converted_into_a_vec() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::Boolean(true)]);

        let values: Vec<bool> = pv.into_vec().expect("convert into Vec<bool>");

        assert_eq!(values, vec![true]);
    }

    #[test]
    fn a_parameterized_value_of_datetimes_can_be_converted_into_a_vec() {
        let datetime = DateTime::from_str("2019-07-27T05:30:30Z").expect("parsing date/time");
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::DateTime(datetime)]);

        let values: Vec<DateTime<Utc>> = pv.into_vec().expect("convert into Vec<DateTime>");

        assert_eq!(values, vec![datetime]);
    }

    #[test]
    fn a_parameterized_value_of_an_array_cant_be_converted_into_a_vec_of_the_wrong_type() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::Integer(1)]);

        let rslt: Option<Vec<f64>> = pv.into_vec();

        assert!(rslt.is_none());
    }
}

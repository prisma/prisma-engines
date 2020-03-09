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
};

#[cfg(feature = "json-1")]
use serde_json::{Number, Value};

#[cfg(feature = "uuid-0_8")]
use uuid::Uuid;

#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Utc};

/// A value we must parameterize for the prepared statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedValue<'a> {
    Null,
    Integer(i64),
    Real(Decimal),
    Text(Cow<'a, str>),
    Enum(Cow<'a, str>),
    Bytes(Cow<'a, [u8]>),
    Boolean(bool),
    Char(char),
    #[cfg(all(feature = "array", feature = "postgresql"))]
    Array(Vec<ParameterizedValue<'a>>),
    #[cfg(feature = "json-1")]
    Json(Value),
    #[cfg(feature = "uuid-0_8")]
    Uuid(Uuid),
    #[cfg(feature = "chrono-0_4")]
    DateTime(DateTime<Utc>),
}

pub(crate) struct Params<'a>(pub(crate) &'a [ParameterizedValue<'a>]);

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

impl<'a> fmt::Display for ParameterizedValue<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParameterizedValue::Null => write!(f, "null"),
            ParameterizedValue::Integer(val) => write!(f, "{}", val),
            ParameterizedValue::Real(val) => write!(f, "{}", val),
            ParameterizedValue::Text(val) => write!(f, "\"{}\"", val),
            ParameterizedValue::Bytes(val) => write!(f, "<{} bytes blob>", val.len()),
            ParameterizedValue::Enum(val) => write!(f, "\"{}\"", val),
            ParameterizedValue::Boolean(val) => write!(f, "{}", val),
            ParameterizedValue::Char(val) => write!(f, "'{}'", val),
            #[cfg(feature = "array")]
            ParameterizedValue::Array(vals) => {
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
            ParameterizedValue::Json(val) => write!(f, "{}", val),
            #[cfg(feature = "uuid-0_8")]
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
            ParameterizedValue::Real(d) => serde_json::to_value(d).unwrap(),
            ParameterizedValue::Text(cow) => Value::String(cow.into_owned()),
            ParameterizedValue::Bytes(bytes) => Value::String(base64::encode(&bytes)),
            ParameterizedValue::Enum(cow) => Value::String(cow.into_owned()),
            ParameterizedValue::Boolean(b) => Value::Bool(b),
            ParameterizedValue::Char(c) => {
                let bytes = [c as u8];
                let s = std::str::from_utf8(&bytes)
                    .expect("interpret byte as UTF-8")
                    .to_string();
                Value::String(s)
            }
            ParameterizedValue::Json(v) => v,
            #[cfg(feature = "array")]
            ParameterizedValue::Array(v) => Value::Array(v.into_iter().map(Value::from).collect()),
            #[cfg(feature = "uuid-0_8")]
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
            ParameterizedValue::Bytes(cow) => std::str::from_utf8(cow.as_ref()).ok(),
            _ => None,
        }
    }

    /// Returns a char if the value is a char, otherwise `None`.
    pub fn as_char(&self) -> Option<char> {
        match self {
            ParameterizedValue::Char(c) => Some(*c),
            _ => None,
        }
    }

    /// Returns a cloned String if the value is text, otherwise `None`.
    pub fn to_string(&self) -> Option<String> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.to_string()),
            ParameterizedValue::Bytes(cow) => std::str::from_utf8(cow.as_ref()).map(|s| s.to_owned()).ok(),
            _ => None,
        }
    }

    /// Transforms the `ParameterizedValue` to a `String` if it's text,
    /// otherwise `None`.
    pub fn into_string(self) -> Option<String> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.into_owned()),
            ParameterizedValue::Bytes(cow) => String::from_utf8(cow.into_owned()).ok(),
            _ => None,
        }
    }

    /// Returns whether this value is the `Bytes` variant.
    pub fn is_bytes(&self) -> bool {
        match self {
            ParameterizedValue::Bytes(_) => true,
            _ => false,
        }
    }

    /// Returns a bytes slice if the value is text or a byte slice, otherwise `None`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.as_ref().as_bytes()),
            ParameterizedValue::Bytes(cow) => Some(cow.as_ref()),
            _ => None,
        }
    }

    /// Returns a cloned `Vec<u8>` if the value is text or a byte slice, otherwise `None`.
    pub fn to_bytes(&self) -> Option<Vec<u8>> {
        match self {
            ParameterizedValue::Text(cow) => Some(cow.to_string().into_bytes()),
            ParameterizedValue::Bytes(cow) => Some(cow.to_owned().into()),
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

    /// Returns a f64 if the value is a real value and the underlying decimal can be converted, otherwise `None`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ParameterizedValue::Real(d) => d.to_f64(),
            _ => None,
        }
    }

    /// Returns a decimal if the value is a real value, otherwise `None`.
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            ParameterizedValue::Real(d) => Some(*d),
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
    #[cfg(feature = "uuid-0_8")]
    pub fn is_uuid(&self) -> bool {
        match self {
            ParameterizedValue::Uuid(_) => true,
            _ => false,
        }
    }

    /// Returns an UUID if the value is of UUID type, otherwise `None`.
    #[cfg(feature = "uuid-0_8")]
    pub fn as_uuid(&self) -> Option<Uuid> {
        match self {
            ParameterizedValue::Uuid(u) => Some(*u),
            _ => None,
        }
    }

    /// `true` if the `ParameterizedValue` is a DateTime.
    #[cfg(feature = "chrono-0_4")]
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
    #[cfg(all(feature = "array", feature = "postgresql"))]
    pub fn into_vec<T>(self) -> Option<Vec<T>>
    where
        // Implement From<ParameterizedValue>
        T: TryFrom<ParameterizedValue<'a>>,
    {
        match self {
            ParameterizedValue::Array(vec) => {
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
    Select(Box<Select<'a>>),
    /// A database function call
    Function(Function<'a>),
    /// A qualified asterisk to a table
    Asterisk(Option<Box<Table<'a>>>),
    /// An operation: sum, sub, mul or div.
    Op(Box<SqlOp<'a>>),
    /// A `VALUES` statement
    Values(Box<Values<'a>>),
}

/// A quick alias to create an asterisk to a table.
///
/// ```rust
/// # use quaint::ast::*;
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
        ParameterizedValue::Integer(i64::try_from(that).unwrap())
    }
}

impl<'a> From<i32> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: i32) -> Self {
        ParameterizedValue::Integer(i64::try_from(that).unwrap())
    }
}

impl<'a> From<&'a [u8]> for ParameterizedValue<'a> {
    fn from(that: &'a [u8]) -> ParameterizedValue<'a> {
        ParameterizedValue::Bytes(that.into())
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for i64 {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<i64, Self::Error> {
        value
            .as_i64()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not an i64")).build())
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for Decimal {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<Decimal, Self::Error> {
        value
            .as_decimal()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a decimal")).build())
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for f64 {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<f64, Self::Error> {
        value
            .as_decimal()
            .and_then(|d| d.to_f64())
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a f64")).build())
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for String {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<String, Self::Error> {
        value
            .into_string()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a string")).build())
    }
}

impl<'a> TryFrom<ParameterizedValue<'a>> for bool {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<bool, Self::Error> {
        value
            .as_bool()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a bool")).build())
    }
}

#[cfg(feature = "chrono-0_4")]
impl<'a> TryFrom<ParameterizedValue<'a>> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: ParameterizedValue<'a>) -> Result<DateTime<Utc>, Self::Error> {
        value
            .as_datetime()
            .ok_or_else(|| Error::builder(ErrorKind::ConversionError("Not a datetime")).build())
    }
}

#[macro_export]
/// Marks a given string as a value. Useful when using a value in calculations,
/// e.g.
///
/// ``` rust
/// # use quaint::{col, val, ast::*, visitor::{Visitor, Sqlite}};
/// let join = "dogs".on(("dogs", "slave_id").equals(Column::from(("cats", "master_id"))));
///
/// let query = Select::from_table("cats")
///     .value(Table::from("cats").asterisk())
///     .value(col!("dogs", "age") - val!(4))
///     .inner_join(join);
///
/// let (sql, params) = Sqlite::build(query);
///
/// assert_eq!(
///     "SELECT `cats`.*, (`dogs`.`age` - ?) FROM `cats` INNER JOIN `dogs` ON `dogs`.`slave_id` = `cats`.`master_id`",
///     sql
/// );
/// ```
macro_rules! val {
    ($val:expr) => {
        DatabaseValue::from($val)
    };
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
parameterized_value!(bool, Boolean);
parameterized_value!(Decimal, Real);

#[cfg(feature = "json-1")]
parameterized_value!(Value, Json);

#[cfg(feature = "uuid-0_8")]
parameterized_value!(Uuid, Uuid);

#[cfg(feature = "chrono-0_4")]
impl<'a> From<DateTime<Utc>> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: DateTime<Utc>) -> Self {
        ParameterizedValue::DateTime(that)
    }
}

impl<'a> From<f64> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: f64) -> Self {
        let dec = Decimal::from_f64(that).expect("f64 is not a Decimal");
        ParameterizedValue::Real(dec)
    }
}

impl<'a> From<f32> for ParameterizedValue<'a> {
    #[inline]
    fn from(that: f32) -> Self {
        let dec = Decimal::from_f32(that).expect("f32 is not a Decimal");
        ParameterizedValue::Real(dec)
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

impl<'a> From<Values<'a>> for DatabaseValue<'a> {
    #[inline]
    fn from(p: Values<'a>) -> Self {
        Self::Values(Box::new(p))
    }
}

impl<'a> From<SqlOp<'a>> for DatabaseValue<'a> {
    #[inline]
    fn from(p: SqlOp<'a>) -> Self {
        Self::Op(Box::new(p))
    }
}

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
        Compare::Between(Box::new(self), Box::new(left.into()), Box::new(right.into()))
    }

    #[inline]
    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
        V: Into<DatabaseValue<'a>>,
    {
        Compare::NotBetween(Box::new(self), Box::new(left.into()), Box::new(right.into()))
    }
}

#[cfg(all(test, feature = "array", feature = "postgresql"))]
mod tests {
    use super::*;
    #[cfg(feature = "chrono-0_4")]
    use std::str::FromStr;

    #[test]
    fn a_parameterized_value_of_ints_can_be_converted_into_a_vec() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::Integer(1)]);

        let values: Vec<i64> = pv.into_vec().expect("convert into Vec<i64>");

        assert_eq!(values, vec![1]);
    }

    #[test]
    fn a_parameterized_value_of_reals_can_be_converted_into_a_vec() {
        let pv = ParameterizedValue::Array(vec![ParameterizedValue::from(1.0)]);

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
    #[cfg(feature = "chrono-0_4")]
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
            rows: Vec::with_capacity(capacity)
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
                None => return None
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

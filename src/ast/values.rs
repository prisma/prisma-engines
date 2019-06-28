use crate::ast::*;
use std::borrow::Cow;

#[cfg(feature = "json-1")]
use serde_json::Value;

#[cfg(feature = "uuid-0_7")]
use uuid::Uuid;

#[cfg(feature = "chrono-0_4")]
use chrono::{DateTime, Utc};

/// A value we must parameterize for the prepared statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedValue<'a> {
    /// A database null
    Null,
    /// An integer value
    Integer(i64),
    /// A floating point value
    Real(f64),
    /// A string value
    Text(Cow<'a, str>),
    /// A boolean value
    Boolean(bool),
    // An array of things.
    #[cfg(feature = "array")]
    Array(Vec<ParameterizedValue<'a>>),
    /// A JSON value
    #[cfg(feature = "json-1")]
    Json(Value),
    #[cfg(feature = "uuid-0_7")]
    Uuid(Uuid),
    #[cfg(feature = "chrono-0_4")]
    DateTime(DateTime<Utc>),
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

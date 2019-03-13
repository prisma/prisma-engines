use crate::ast::*;

/// A value we must parameterize for the prepared statement.
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedValue {
    /// A database null
    Null,
    /// An integer value
    Integer(i64),
    /// A floating point value
    Real(f64),
    /// A string value
    Text(String),
    /// a boolean value
    Boolean(bool),
}

/// A value we can compare and use in database queries.
#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseValue {
    /// Anything that we must parameterize before querying
    Parameterized(ParameterizedValue),
    /// A database column
    Column(Box<Column>),
    /// Data in a row form, e.g. (1, 2, 3)
    Row(Row),
    /// A nested `SELECT` statement
    Select(Select),
    /// A database function call
    Function(Function),
    /// A qualified asterisk to a table
    Asterisk(Option<Table>),
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
pub fn asterisk() -> DatabaseValue {
    DatabaseValue::Asterisk(None)
}

impl Into<ParameterizedValue> for &str {
    #[inline]
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Text(self.to_string())
    }
}

impl Into<ParameterizedValue> for String {
    #[inline]
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Text(self)
    }
}

impl Into<ParameterizedValue> for i64 {
    #[inline]
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Integer(self)
    }
}

impl Into<ParameterizedValue> for f64 {
    #[inline]
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Real(self)
    }
}

impl Into<ParameterizedValue> for bool {
    #[inline]
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Boolean(self)
    }
}

impl<'a> Into<DatabaseValue> for &'a str {
    #[inline]
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for String {
    #[inline]
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for i64 {
    #[inline]
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for f64 {
    #[inline]
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for bool {
    #[inline]
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for ParameterizedValue {
    #[inline]
    fn into(self) -> DatabaseValue {
        DatabaseValue::Parameterized(self)
    }
}

impl Into<DatabaseValue> for Row {
    #[inline]
    fn into(self) -> DatabaseValue {
        DatabaseValue::Row(self)
    }
}

impl Into<DatabaseValue> for Function {
    #[inline]
    fn into(self) -> DatabaseValue {
        DatabaseValue::Function(self)
    }
}

impl<T> From<Vec<T>> for DatabaseValue
where
    T: Into<DatabaseValue>,
{
    #[inline]
    fn from(v: Vec<T>) -> DatabaseValue {
        let row: Row = v.into();
        row.into()
    }
}

impl Comparable for DatabaseValue {
    #[inline]
    fn equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::Equals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn not_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::NotEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn less_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::LessThan(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn less_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::LessThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn greater_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::GreaterThan(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::GreaterThanOrEquals(Box::new(self), Box::new(comparison.into()))
    }

    #[inline]
    fn in_selection<T>(self, selection: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::In(Box::new(self), Box::new(selection.into()))
    }

    #[inline]
    fn not_in_selection<T>(self, selection: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::NotIn(Box::new(self), Box::new(selection.into()))
    }

    #[inline]
    fn like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        Compare::Like(Box::new(self), pattern.into())
    }

    #[inline]
    fn not_like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        Compare::NotLike(Box::new(self), pattern.into())
    }

    #[inline]
    fn begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        Compare::BeginsWith(Box::new(self), pattern.into())
    }

    #[inline]
    fn not_begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        Compare::NotBeginsWith(Box::new(self), pattern.into())
    }

    #[inline]
    fn ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        Compare::EndsInto(Box::new(self), pattern.into())
    }

    #[inline]
    fn not_ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        Compare::NotEndsInto(Box::new(self), pattern.into())
    }

    #[inline]
    fn is_null(self) -> Compare {
        Compare::Null(Box::new(self))
    }

    #[inline]
    fn is_not_null(self) -> Compare {
        Compare::NotNull(Box::new(self))
    }

    #[inline]
    fn between<T, V>(self, left: T, right: V) -> Compare
    where
        T: Into<DatabaseValue>,
        V: Into<DatabaseValue>,
    {
        Compare::Between(
            Box::new(self),
            Box::new(left.into()),
            Box::new(right.into()),
        )
    }

    #[inline]
    fn not_between<T, V>(self, left: T, right: V) -> Compare
    where
        T: Into<DatabaseValue>,
        V: Into<DatabaseValue>,
    {
        Compare::NotBetween(
            Box::new(self),
            Box::new(left.into()),
            Box::new(right.into()),
        )
    }
}

use crate::ast::{Column, Comparable, Compare, Row, Select};

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterizedValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DatabaseValue {
    Parameterized(ParameterizedValue),
    Column(Column),
    Row(Row),
    Select(Select),
}

impl<'a> Into<ParameterizedValue> for &'a str {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Text(self.to_string())
    }
}

impl Into<ParameterizedValue> for String {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Text(self)
    }
}

impl Into<ParameterizedValue> for i64 {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Integer(self)
    }
}

impl Into<ParameterizedValue> for f64 {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Real(self)
    }
}

impl Into<ParameterizedValue> for bool {
    fn into(self) -> ParameterizedValue {
        ParameterizedValue::Boolean(self)
    }
}

impl<'a> Into<DatabaseValue> for &'a str {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for String {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for i64 {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for f64 {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for bool {
    fn into(self) -> DatabaseValue {
        let val: ParameterizedValue = self.into();
        DatabaseValue::Parameterized(val)
    }
}

impl Into<DatabaseValue> for ParameterizedValue {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Parameterized(self)
    }
}

impl Into<DatabaseValue> for Row {
    fn into(self) -> DatabaseValue {
        DatabaseValue::Row(self)
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
    fn in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::In(Box::new(self), Box::new(Row::from(selection).into()))
    }

    #[inline]
    fn not_in_selection<T>(self, selection: Vec<T>) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        Compare::NotIn(Box::new(self), Box::new(Row::from(selection).into()))
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
}

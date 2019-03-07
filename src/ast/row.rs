use crate::ast::{Comparable, Compare, DatabaseValue};

/// A collection of values surrounded by parentheses.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Row {
    pub values: Vec<DatabaseValue>,
}

impl Row {
    pub fn new() -> Self {
        Row { values: Vec::new() }
    }

    pub fn add<T>(mut self, value: T) -> Self
    where
        T: Into<DatabaseValue>,
    {
        self.values.push(value.into());
        self
    }
}

impl<T> From<Vec<T>> for Row
where
    T: Into<DatabaseValue>,
{
    fn from(vector: Vec<T>) -> Row {
        vector
            .into_iter()
            .fold(Row::new(), |row, v| row.add(v.into()))
    }
}

impl<A, B> From<(A, B)> for Row
where
    A: Into<DatabaseValue>,
    B: Into<DatabaseValue>,
{
    fn from(vals: (A, B)) -> Row {
        Row::new().add(vals.0).add(vals.1)
    }
}

impl<A, B, C> From<(A, B, C)> for Row
where
    A: Into<DatabaseValue>,
    B: Into<DatabaseValue>,
    C: Into<DatabaseValue>,
{
    fn from(vals: (A, B, C)) -> Row {
        Row::new().add(vals.0).add(vals.1).add(vals.2)
    }
}

impl<A, B, C, D> From<(A, B, C, D)> for Row
where
    A: Into<DatabaseValue>,
    B: Into<DatabaseValue>,
    C: Into<DatabaseValue>,
    D: Into<DatabaseValue>,
{
    fn from(vals: (A, B, C, D)) -> Row {
        Row::new().add(vals.0).add(vals.1).add(vals.2).add(vals.3)
    }
}

impl<A, B, C, D, E> From<(A, B, C, D, E)> for Row
where
    A: Into<DatabaseValue>,
    B: Into<DatabaseValue>,
    C: Into<DatabaseValue>,
    D: Into<DatabaseValue>,
    E: Into<DatabaseValue>,
{
    fn from(vals: (A, B, C, D, E)) -> Row {
        Row::new()
            .add(vals.0)
            .add(vals.1)
            .add(vals.2)
            .add(vals.3)
            .add(vals.4)
    }
}

impl Comparable for Row {
    fn equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.equals(comparison)
    }

    fn not_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_equals(comparison)
    }

    fn less_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_equals(comparison)
    }

    fn less_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.less_than_or_equals(comparison)
    }

    fn greater_than<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.greater_than(comparison)
    }

    fn greater_than_or_equals<T>(self, comparison: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.greater_than_or_equals(comparison)
    }

    fn in_selection<T>(self, selection: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.in_selection(selection)
    }

    fn not_in_selection<T>(self, selection: T) -> Compare
    where
        T: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_in_selection(selection)
    }

    fn like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.like(pattern)
    }

    fn not_like<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_like(pattern)
    }

    fn begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.begins_with(pattern)
    }

    fn not_begins_with<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_begins_with(pattern)
    }

    fn ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.ends_into(pattern)
    }

    fn not_ends_into<T>(self, pattern: T) -> Compare
    where
        T: Into<String>,
    {
        let value: DatabaseValue = self.into();
        value.not_ends_into(pattern)
    }

    fn is_null(self) -> Compare {
        let value: DatabaseValue = self.into();
        value.is_null()
    }

    fn is_not_null(self) -> Compare {
        let value: DatabaseValue = self.into();
        value.is_not_null()
    }

    fn between<T, V>(self, left: T, right: V) -> Compare
    where
        T: Into<DatabaseValue>,
        V: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.between(left, right)
    }

    fn not_between<T, V>(self, left: T, right: V) -> Compare
    where
        T: Into<DatabaseValue>,
        V: Into<DatabaseValue>,
    {
        let value: DatabaseValue = self.into();
        value.not_between(left, right)
    }
}

use crate::ast::{Comparable, Compare, DatabaseValue};
use std::borrow::Cow;

/// A collection of values surrounded by parentheses.
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Row<'a> {
    pub values: Vec<DatabaseValue<'a>>,
}

impl<'a> Row<'a> {
    #[inline]
    pub fn new() -> Self {
        Row { values: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Row { values: Vec::with_capacity(capacity) }
    }

    pub fn push<T>(mut self, value: T) -> Self
    where
        T: Into<DatabaseValue<'a>>,
    {
        self.values.push(value.into());
        self
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<'a, T> From<Vec<T>> for Row<'a>
where
    T: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from(vector: Vec<T>) -> Row<'a> {
        vector.into_iter().fold(Row::new(), |row, v| row.push(v.into()))
    }
}

impl<'a, A> From<(A,)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from((val,): (A,)) -> Self {
        Row::with_capacity(1).push(val)
    }
}

impl<'a, A, B> From<(A, B)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
    B: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from(vals: (A, B)) -> Self {
        Row::with_capacity(2).push(vals.0).push(vals.1)
    }
}

impl<'a, A, B, C> From<(A, B, C)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
    B: Into<DatabaseValue<'a>>,
    C: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from(vals: (A, B, C)) -> Self {
        Row::with_capacity(3).push(vals.0).push(vals.1).push(vals.2)
    }
}

impl<'a, A, B, C, D> From<(A, B, C, D)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
    B: Into<DatabaseValue<'a>>,
    C: Into<DatabaseValue<'a>>,
    D: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from(vals: (A, B, C, D)) -> Self {
        Row::with_capacity(4).push(vals.0).push(vals.1).push(vals.2).push(vals.3)
    }
}

impl<'a, A, B, C, D, E> From<(A, B, C, D, E)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
    B: Into<DatabaseValue<'a>>,
    C: Into<DatabaseValue<'a>>,
    D: Into<DatabaseValue<'a>>,
    E: Into<DatabaseValue<'a>>,
{
    #[inline]
    fn from(vals: (A, B, C, D, E)) -> Self {
        Row::with_capacity(5)
            .push(vals.0)
            .push(vals.1)
            .push(vals.2)
            .push(vals.3)
            .push(vals.4)
    }
}

impl<'a> Comparable<'a> for Row<'a> {
    #[inline]
    fn equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.equals(comparison)
    }

    #[inline]
    fn not_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.not_equals(comparison)
    }

    #[inline]
    fn less_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.less_than(comparison)
    }

    #[inline]
    fn less_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.less_than_or_equals(comparison)
    }

    #[inline]
    fn greater_than<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.greater_than(comparison)
    }

    #[inline]
    fn greater_than_or_equals<T>(self, comparison: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.greater_than_or_equals(comparison)
    }

    #[inline]
    fn in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.in_selection(selection)
    }

    #[inline]
    fn not_in_selection<T>(self, selection: T) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.not_in_selection(selection)
    }

    #[inline]
    fn like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.like(pattern)
    }

    #[inline]
    fn not_like<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.not_like(pattern)
    }

    #[inline]
    fn begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.begins_with(pattern)
    }

    #[inline]
    fn not_begins_with<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.not_begins_with(pattern)
    }

    #[inline]
    fn ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.ends_into(pattern)
    }

    #[inline]
    fn not_ends_into<T>(self, pattern: T) -> Compare<'a>
    where
        T: Into<Cow<'a, str>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.not_ends_into(pattern)
    }

    #[inline]
    fn is_null(self) -> Compare<'a> {
        let value: DatabaseValue<'a> = self.into();
        value.is_null()
    }

    #[inline]
    fn is_not_null(self) -> Compare<'a> {
        let value: DatabaseValue<'a> = self.into();
        value.is_not_null()
    }

    #[inline]
    fn between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
        V: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.between(left, right)
    }

    #[inline]
    fn not_between<T, V>(self, left: T, right: V) -> Compare<'a>
    where
        T: Into<DatabaseValue<'a>>,
        V: Into<DatabaseValue<'a>>,
    {
        let value: DatabaseValue<'a> = self.into();
        value.not_between(left, right)
    }
}

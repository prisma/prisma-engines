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

    pub fn pop(&mut self) -> Option<DatabaseValue<'a>> {
        self.values.pop()
    }

    pub fn push<T>(&mut self, value: T)
    where
        T: Into<DatabaseValue<'a>>,
    {
        self.values.push(value.into());
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl<'a, T> From<Vec<T>> for Row<'a>
where
    T: Into<DatabaseValue<'a>>,
{
    fn from(vector: Vec<T>) -> Row<'a> {
        let mut row = Row::with_capacity(vector.len());

        for v in vector.into_iter() {
            row.push(v.into());
        }

        row
    }
}

impl<'a, A> From<(A,)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
{
    fn from((val,): (A,)) -> Self {
        let mut row = Row::with_capacity(1);
        row.push(val);
        row
    }
}

impl<'a, A, B> From<(A, B)> for Row<'a>
where
    A: Into<DatabaseValue<'a>>,
    B: Into<DatabaseValue<'a>>,
{
    fn from(vals: (A, B)) -> Self {
        let mut row = Row::with_capacity(2);

        row.push(vals.0);
        row.push(vals.1);

        row
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
        let mut row = Row::with_capacity(3);

        row.push(vals.0);
        row.push(vals.1);
        row.push(vals.2);

        row
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
        let mut row = Row::with_capacity(4);

        row.push(vals.0);
        row.push(vals.1);
        row.push(vals.2);
        row.push(vals.3);

        row
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
        let mut row = Row::with_capacity(5);

        row.push(vals.0);
        row.push(vals.1);
        row.push(vals.2);
        row.push(vals.3);
        row.push(vals.4);

        row
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

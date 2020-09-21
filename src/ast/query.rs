use crate::ast::{Delete, Insert, Select, Union, Update};
use std::borrow::Cow;

use super::IntoCommonTableExpression;

/// A database query
#[derive(Debug, Clone, PartialEq)]
pub enum Query<'a> {
    /// Query for fetching data. E.g. the `SELECT` query.
    Select(Box<Select<'a>>),
    Insert(Box<Insert<'a>>),
    Update(Box<Update<'a>>),
    Delete(Box<Delete<'a>>),
    Union(Box<Union<'a>>),
    Raw(Cow<'a, str>),
}

impl<'a, T> From<T> for Query<'a>
where
    T: Into<Cow<'a, str>>,
{
    fn from(t: T) -> Self {
        Query::Raw(t.into())
    }
}

impl<'a> Query<'a> {
    pub fn is_select(&self) -> bool {
        matches!(self, Query::Select(_))
    }

    pub fn is_insert(&self) -> bool {
        matches!(self, Query::Insert(_))
    }

    pub fn is_update(&self) -> bool {
        matches!(self, Query::Update(_))
    }

    pub fn is_delete(&self) -> bool {
        matches!(self, Query::Delete(_))
    }

    pub fn is_union(&self) -> bool {
        matches!(self, Query::Union(_))
    }
}

/// A database query that only returns data without modifying anything.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectQuery<'a> {
    Select(Box<Select<'a>>),
    Union(Box<Union<'a>>),
}

impl<'a> SelectQuery<'a> {
    /// Finds all named values or columns from the selection.
    pub fn named_selection(&self) -> Vec<String> {
        match self {
            Self::Select(s) => s.named_selection(),
            Self::Union(u) => u.named_selection(),
        }
    }
}

impl<'a> From<Select<'a>> for SelectQuery<'a> {
    fn from(s: Select<'a>) -> Self {
        Self::Select(Box::new(s))
    }
}

impl<'a> From<Union<'a>> for SelectQuery<'a> {
    fn from(u: Union<'a>) -> Self {
        Self::Union(Box::new(u))
    }
}

impl<'a> From<SelectQuery<'a>> for Query<'a> {
    fn from(sq: SelectQuery<'a>) -> Self {
        match sq {
            SelectQuery::Select(s) => Query::Select(s),
            SelectQuery::Union(u) => Query::Union(u),
        }
    }
}

impl<'a> IntoCommonTableExpression<'a> for SelectQuery<'a> {}

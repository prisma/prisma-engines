use crate::ast::{Delete, Insert, Select, UnionAll, Update};
use std::borrow::Cow;

/// A database query
#[derive(Debug, Clone, PartialEq)]
pub enum Query<'a> {
    /// Query for fetching data. E.g. the `SELECT` query.
    Select(Select<'a>),
    Insert(Box<Insert<'a>>),
    Update(Box<Update<'a>>),
    Delete(Box<Delete<'a>>),
    UnionAll(UnionAll<'a>),
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
        if let Query::Select(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_insert(&self) -> bool {
        if let Query::Insert(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_update(&self) -> bool {
        if let Query::Update(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_delete(&self) -> bool {
        if let Query::Delete(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_union_all(&self) -> bool {
        if let Query::UnionAll(_) = self {
            true
        } else {
            false
        }
    }
}

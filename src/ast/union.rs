use crate::ast::{Query, Select};
use std::fmt;

#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) enum UnionType {
    All,
    Distinct,
}

impl fmt::Display for UnionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnionType::All => write!(f, "UNION ALL"),
            UnionType::Distinct => write!(f, "UNION"),
        }
    }
}

/// A builder for a `UNION`s over multiple `SELECT` statements.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct Union<'a> {
    pub(crate) selects: Vec<Select<'a>>,
    pub(crate) types: Vec<UnionType>,
}

impl<'a> From<Union<'a>> for Query<'a> {
    fn from(ua: Union<'a>) -> Self {
        Query::Union(ua)
    }
}

impl<'a> Union<'a> {
    pub fn new(q: Select<'a>) -> Self {
        Self {
            selects: vec![q],
            types: Vec::new(),
        }
    }

    /// Creates a union with previous selection and the given `SELECT`
    /// statement, allowing duplicates.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let s1 = Select::default().value(1);
    /// let s2 = Select::default().value(2);
    /// let (sql, params) = Sqlite::build(Union::new(s1).all(s2));
    ///
    /// assert_eq!("(SELECT ?) UNION ALL (SELECT ?)", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(1),
    ///     Value::from(2)
    /// ], params);
    /// ```
    pub fn all(mut self, q: Select<'a>) -> Self {
        self.selects.push(q);
        self.types.push(UnionType::All);
        self
    }

    /// Creates a union with previous selection and the given `SELECT`
    /// statement, selecting only distinct values.
    ///
    /// ```rust
    /// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
    /// let s1 = Select::default().value(1);
    /// let s2 = Select::default().value(2);
    /// let (sql, params) = Sqlite::build(Union::new(s1).distinct(s2));
    ///
    /// assert_eq!("(SELECT ?) UNION (SELECT ?)", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(1),
    ///     Value::from(2)
    /// ], params);
    /// ```
    pub fn distinct(mut self, q: Select<'a>) -> Self {
        self.selects.push(q);
        self.types.push(UnionType::Distinct);
        self
    }
}

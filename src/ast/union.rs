use crate::ast::{Expression, Query, Select};
use std::{collections::BTreeSet, fmt};

use super::IntoCommonTableExpression;

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
        Query::Union(Box::new(ua))
    }
}

impl<'a> From<Union<'a>> for Expression<'a> {
    fn from(uaua: Union<'a>) -> Self {
        Expression::union(uaua)
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
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let s1 = Select::default().value(1);
    /// let s2 = Select::default().value(2);
    /// let (sql, params) = Sqlite::build(Union::new(s1).all(s2))?;
    ///
    /// assert_eq!("SELECT ? UNION ALL SELECT ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(1),
    ///     Value::from(2)
    /// ], params);
    /// # Ok(())
    /// # }
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
    /// # fn main() -> Result<(), quaint::error::Error> {
    /// let s1 = Select::default().value(1);
    /// let s2 = Select::default().value(2);
    /// let (sql, params) = Sqlite::build(Union::new(s1).distinct(s2))?;
    ///
    /// assert_eq!("SELECT ? UNION SELECT ?", sql);
    ///
    /// assert_eq!(vec![
    ///     Value::from(1),
    ///     Value::from(2)
    /// ], params);
    /// # Ok(())
    /// # }
    /// ```
    pub fn distinct(mut self, q: Select<'a>) -> Self {
        self.selects.push(q);
        self.types.push(UnionType::Distinct);
        self
    }

    /// A list of item names in the queries, skipping the anonymous values or
    /// columns.
    pub(crate) fn named_selection(&self) -> Vec<String> {
        self.selects
            .iter()
            .fold(BTreeSet::new(), |mut acc, select| {
                for name in select.named_selection() {
                    acc.insert(name);
                }

                acc
            })
            .into_iter()
            .collect()
    }

    /// Finds all comparisons between tuples and selects in the queries and
    /// converts them to common table expressions for making the query
    /// compatible with databases not supporting tuples.
    pub(crate) fn convert_tuple_selects_into_ctes(mut self, level: &mut usize) -> Self {
        let mut converted = Vec::with_capacity(self.selects.len());

        for select in self.selects.drain(0..) {
            converted.push(select.convert_tuple_select_to_cte(level));
        }

        self.selects = converted;

        self
    }
}

impl<'a> IntoCommonTableExpression<'a> for Union<'a> {}

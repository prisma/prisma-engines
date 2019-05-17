use crate::ast::{Query, Select};

/// A builder for a `UNION ALL` over multiple `SELECT` statements.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct UnionAll(pub Vec<Select>);

impl From<Select> for UnionAll {
    fn from(q: Select) -> Self {
        UnionAll(vec![q])
    }
}

impl From<UnionAll> for Query {
    #[inline]
    fn from(ua: UnionAll) -> Query {
        Query::UnionAll(ua)
    }
}

impl UnionAll {
    /// Creates a union with previous and given `SELECT` statement.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let s1 = Select::default().value(1);
    /// let s2 = Select::default().value(2);
    /// let (sql, params) = Sqlite::build(UnionAll::from(s1).add(s2));
    ///
    /// assert_eq!("(SELECT ?) UNION ALL (SELECT ?)", sql);
    ///
    /// assert_eq!(vec![
    ///     ParameterizedValue::from(1),
    ///     ParameterizedValue::from(2)
    /// ], params);
    /// ```
    pub fn add(mut self, q: Select) -> Self {
        self.0.push(q);
        self
    }
}

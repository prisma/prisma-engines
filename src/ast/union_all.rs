use crate::ast::{Query, Select};

/// A builder for a `UNION ALL` over multiple `SELECT` statements.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct UnionAll<'a>(pub Vec<Select<'a>>);

impl<'a> From<Select<'a>> for UnionAll<'a> {
    fn from(q: Select<'a>) -> Self {
        UnionAll(vec![q])
    }
}

impl<'a> From<UnionAll<'a>> for Query<'a> {
    #[inline]
    fn from(ua: UnionAll<'a>) -> Self {
        Query::UnionAll(ua)
    }
}

impl<'a> UnionAll<'a> {
    /// Creates a union with previous and given `SELECT` statement.
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let s1 = Select::default().value(1);
    /// let s2 = Select::default().value(2);
    /// let (sql, params) = Sqlite::build(UnionAll::from(s1).union_all(s2));
    ///
    /// assert_eq!("(SELECT ?) UNION ALL (SELECT ?)", sql);
    ///
    /// assert_eq!(vec![
    ///     ParameterizedValue::from(1),
    ///     ParameterizedValue::from(2)
    /// ], params);
    /// ```
    pub fn union_all(mut self, q: Select<'a>) -> Self {
        self.0.push(q);
        self
    }
}

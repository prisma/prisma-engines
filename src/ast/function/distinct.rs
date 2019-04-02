use crate::ast::{Column, DatabaseValue};

#[derive(Debug, Clone, PartialEq)]
pub struct Distinct {
    pub(crate) exprs: Vec<DatabaseValue>,
}

impl Distinct {
    /// Add another expression to a distinct statement
    ///
    /// ```rust
    /// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
    /// let fun = distinct(Column::from(("users", "id")))
    ///     .distinct(Column::from(("users", "name")));
    ///
    /// let query = Select::from_table("users").value(fun);
    /// let (sql, _) = Sqlite::build(query);
    /// assert_eq!("SELECT DISTINCT(`users`.`id`, `users`.`name`) FROM `users` LIMIT -1", sql);
    /// ```
    pub fn distinct<T>(mut self, expr: T) -> Distinct
    where
        T: Into<DatabaseValue>,
    {
        self.exprs.push(expr.into());
        self
    }
}

/// Select distinct rows by given expressions.
///
/// ```rust
/// # use prisma_query::{ast::*, visitor::{Visitor, Sqlite}};
/// let query = Select::from_table("users").value(distinct("name"));
/// let (sql, _) = Sqlite::build(query);
/// assert_eq!("SELECT DISTINCT(`name`) FROM `users` LIMIT -1", sql);
/// ```
#[inline]
pub fn distinct<T>(expr: T) -> Distinct
where
    T: Into<Column>,
{
    let column: Column = expr.into();

    Distinct {
        exprs: vec![column.into()],
    }
}

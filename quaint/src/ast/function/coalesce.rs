use super::Function;
use crate::ast::Expression;

#[derive(Debug, Clone, PartialEq)]
/// Returns the first non-null expression
pub struct Coalesce<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
}

/// Returns the first non-null argument
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let exprs: Vec<Expression> = vec![
///   Column::from(("users", "company")).into(),
///   Value::text("Individual").into(),
/// ];
/// let query = Select::from_table("users").value(coalesce(exprs));
/// let (sql, params) = Sqlite::build(query)?;
/// assert_eq!("SELECT COALESCE(`users`.`company`, ?) FROM `users`", sql);
/// assert_eq!(vec![Value::text("Individual")], params);
/// # Ok(())
/// # }
/// ```
pub fn coalesce<'a, T, V>(exprs: V) -> Function<'a>
where
    T: Into<Expression<'a>>,
    V: Into<Vec<T>>,
{
    let fun = Coalesce {
        exprs: exprs.into().into_iter().map(|e| e.into()).collect(),
    };

    fun.into()
}

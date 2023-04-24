use super::Function;
use crate::ast::Expression;

/// A representation of the `Concat` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct Concat<'a> {
    pub(crate) exprs: Vec<Expression<'a>>,
}

/// Concat several expressions.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(concat(vec!["firstname", "lastname"]));
/// let (sql, params) = Sqlite::build(query)?;
/// assert_eq!("SELECT (? || ?) FROM `users`", sql);
/// assert_eq!(params, vec![Value::from("firstname"), Value::from("lastname")]);
/// # Ok(())
/// # }
/// ```
pub fn concat<'a, T>(exprs: Vec<T>) -> Function<'a>
where
    T: Into<Expression<'a>>,
{
    let fun = Concat {
        exprs: exprs.into_iter().map(Into::into).collect(),
    };

    fun.into()
}

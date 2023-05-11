use super::Function;
use crate::ast::Expression;

#[derive(Debug, Clone, PartialEq)]
pub struct JsonUnquote<'a> {
    pub(crate) expr: Box<Expression<'a>>,
}

/// Converts a JSON expression into string and unquotes it.
///
/// ```rust
/// # use quaint::{ast::*, col, visitor::{Visitor, Mysql}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(json_unquote(col!("json")));
/// let (sql, _) = Mysql::build(query)?;
/// assert_eq!("SELECT JSON_UNQUOTE(`json`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn json_unquote<'a, E>(expr: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = JsonUnquote {
        expr: Box::new(expr.into()),
    };

    fun.into()
}

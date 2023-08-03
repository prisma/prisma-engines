use super::Function;
use crate::ast::Expression;

/// A represention of the `ST_AsText` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct GeomAsText<'a> {
    pub(crate) expression: Box<Expression<'a>>,
}

/// Read the geometry expression into a EWKT string.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(geom_as_text(Column::from("location")));
/// let (sql, _) = Sqlite::build(query)?;
///
/// assert_eq!("SELECT AsEWKT(`location`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn geom_as_text<'a, E>(expression: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = GeomAsText {
        expression: Box::new(expression.into()),
    };

    fun.into()
}

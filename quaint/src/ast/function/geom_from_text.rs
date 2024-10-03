use super::Function;
use crate::ast::Expression;

/// A represention of the `ST_AsText` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct GeomFromText<'a> {
    pub(crate) wkt_expression: Box<Expression<'a>>,
    pub(crate) srid_expression: Option<Box<Expression<'a>>>,
    pub(crate) geography: bool,
}

/// Write a WKT geometry value using built-in database conversion.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Insert::single_into("users").value("location", geom_from_text("POINT(0 0)", 4326, false));
/// let (sql, params) = Sqlite::build(query)?;
///
/// assert_eq!("INSERT INTO `users` (`location`) VALUES (ST_GeomFromText(?,?))", sql);
///
/// assert_eq!(vec![
///    Value::from("POINT(0 0)"),
///    Value::from(4326)
///  ], params);
/// # Ok(())
/// # }
/// ```
pub fn geom_from_text<'a, G, S>(wkt_expression: G, srid_expression: Option<S>, geography: bool) -> Function<'a>
where
    G: Into<Expression<'a>>,
    S: Into<Expression<'a>>,
{
    let fun = GeomFromText {
        wkt_expression: Box::new(wkt_expression.into()),
        srid_expression: srid_expression.map(|s| Box::new(s.into())),
        geography,
    };

    fun.into()
}

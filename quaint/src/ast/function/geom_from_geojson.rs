use super::Function;
use crate::ast::Expression;

/// A represention of the `ST_AsText` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct GeomFromGeoJson<'a> {
    pub(crate) expression: Box<Expression<'a>>,
}

/// Write a GeoJson geometry value using built-in database conversion.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Insert::single_into("users").value("location", geom_from_geojson("POINT(0 0)"));
/// let (sql, params) = Sqlite::build(query)?;
///
/// assert_eq!("INSERT INTO `users` (`location`) VALUES (GeomFromGeoJSON(?))", sql);
///
/// assert_eq!(vec![
///    Value::from("POINT(0 0)"),
///  ], params);
/// # Ok(())
/// # }
/// ```
pub fn geom_from_geojson<'a, G>(expression: G) -> Function<'a>
where
    G: Into<Expression<'a>>,
{
    let fun = GeomFromGeoJson {
        expression: Box::new(expression.into()),
    };

    fun.into()
}

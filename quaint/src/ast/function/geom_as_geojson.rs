use super::Function;
use crate::ast::Expression;

/// A represention of the `ST_AsGeoJSON` function in the database.
#[derive(Debug, Clone, PartialEq)]
pub struct GeomAsGeoJson<'a> {
    pub(crate) expression: Box<Expression<'a>>,
}

/// Read the geometry expression into a GeoJson object.
///
/// ```rust
/// # use quaint::{ast::*, visitor::{Visitor, Sqlite}};
/// # fn main() -> Result<(), quaint::error::Error> {
/// let query = Select::from_table("users").value(geom_as_geojson(Column::from("location")));
/// let (sql, _) = Sqlite::build(query)?;
///
/// assert_eq!("SELECT AsGeoJSON(`location`) FROM `users`", sql);
/// # Ok(())
/// # }
/// ```
pub fn geom_as_geojson<'a, E>(expression: E) -> Function<'a>
where
    E: Into<Expression<'a>>,
{
    let fun = GeomAsGeoJson {
        expression: Box::new(expression.into()),
    };

    fun.into()
}

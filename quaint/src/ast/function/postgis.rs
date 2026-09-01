use super::Function;
use crate::ast::Expression;

/// A PostGIS function call. Used to render spatial SQL expressions (`ST_*`) with each argument
/// going through the regular parameterized-expression path of the visitor, so no user input
/// is ever interpolated into raw SQL text.
#[derive(Debug, Clone, PartialEq)]
pub struct PostgisFunction<'a> {
    pub(crate) name: &'static str,
    pub(crate) args: Vec<Expression<'a>>,
}

impl<'a> PostgisFunction<'a> {
    pub(crate) fn build(name: &'static str, args: Vec<Expression<'a>>) -> Function<'a> {
        Function {
            typ_: super::FunctionType::Postgis(Self { name, args }),
            alias: None,
        }
    }

    pub fn name(&self) -> &'static str {
        self.name
    }

    pub fn args(&self) -> &[Expression<'a>] {
        &self.args
    }
}

/// `ST_DWithin(geom_a, geom_b, distance)` - returns `true` when the geometries are within
/// `distance` meters/units of each other.
pub fn st_dwithin<'a, A, B, C>(geom_a: A, geom_b: B, distance: C) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
    C: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_DWithin", vec![geom_a.into(), geom_b.into(), distance.into()])
}

/// `ST_Within(geom_a, geom_b)` - returns `true` if `geom_a` is completely contained inside
/// `geom_b`.
pub fn st_within<'a, A, B>(geom_a: A, geom_b: B) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_Within", vec![geom_a.into(), geom_b.into()])
}

/// `ST_Intersects(geom_a, geom_b)` - returns `true` if the geometries share any point.
pub fn st_intersects<'a, A, B>(geom_a: A, geom_b: B) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_Intersects", vec![geom_a.into(), geom_b.into()])
}

/// `ST_Distance(geom_a, geom_b)` - returns the minimum distance between the geometries.
pub fn st_distance<'a, A, B>(geom_a: A, geom_b: B) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_Distance", vec![geom_a.into(), geom_b.into()])
}

/// `ST_GeomFromText(wkt, srid)` - parses a WKT string into a geometry with the given SRID.
pub fn st_geom_from_text<'a, A, B>(wkt: A, srid: B) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_GeomFromText", vec![wkt.into(), srid.into()])
}

/// `ST_MakePoint(x, y)` - constructs a 2D point.
pub fn st_make_point<'a, A, B>(x: A, y: B) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_MakePoint", vec![x.into(), y.into()])
}

/// `ST_SetSRID(geom, srid)` - assigns/overrides the SRID of a geometry without reprojecting.
pub fn st_set_srid<'a, A, B>(geom: A, srid: B) -> Function<'a>
where
    A: Into<Expression<'a>>,
    B: Into<Expression<'a>>,
{
    PostgisFunction::build("ST_SetSRID", vec![geom.into(), srid.into()])
}

/// `geography(geom)` - PostGIS conversion from `geometry` to `geography`. Equivalent to the
/// `::geography` cast but expressible inside the Function AST so the operand stays a regular
/// parameterized expression.
pub fn geography_cast<'a, A>(geom: A) -> Function<'a>
where
    A: Into<Expression<'a>>,
{
    PostgisFunction::build("geography", vec![geom.into()])
}

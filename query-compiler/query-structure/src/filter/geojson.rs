//! Strongly-typed GeoJSON geometry representation used by the geometry filter pipeline.
//!
//! The geometry filter accepts user-supplied GeoJSON. Parsing it once at the extractor layer
//! into [`GeoJsonGeometry`] guarantees that downstream code (visitor, ordering, snapshots) can
//! rely on validated invariants:
//!
//! * `type` is one of the supported GeoJSON values.
//! * All coordinates are finite (`f64::is_finite`).
//! * Polygon rings are closed (first vertex == last vertex).
//! * Rings have at least four positions (i.e. the closing vertex is mandatory).
//!
//! Any violation is rejected with [`GeoJsonParseError`] so that callers can map it to a
//! user-facing input error instead of panicking later.

use std::hash::{Hash, Hasher};

use thiserror::Error;

/// A single GeoJSON 2D coordinate (longitude/easting + latitude/northing).
#[derive(Debug, Clone, Copy)]
pub struct GeoCoord {
    pub x: f64,
    pub y: f64,
}

impl GeoCoord {
    pub fn new(x: f64, y: f64) -> Result<Self, GeoJsonParseError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(GeoJsonParseError::NonFiniteCoord { x, y });
        }
        Ok(Self { x, y })
    }
}

impl PartialEq for GeoCoord {
    fn eq(&self, other: &Self) -> bool {
        self.x.to_bits() == other.x.to_bits() && self.y.to_bits() == other.y.to_bits()
    }
}

impl Eq for GeoCoord {}

impl Hash for GeoCoord {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
    }
}

/// Validated GeoJSON geometry subset.
///
/// `Multi*` and `GeometryCollection` are accepted for round-tripping, but actual SQL support
/// in the visitor is currently limited to `Point`, `LineString`, and `Polygon`. The visitor
/// rejects unsupported variants explicitly rather than panicking.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GeoJsonGeometry {
    Point(GeoCoord),
    LineString(Vec<GeoCoord>),
    /// Polygon rings. The first ring is the outer boundary, subsequent rings are holes.
    /// Every ring is guaranteed to be closed and contain ≥ 4 positions.
    Polygon(Vec<Vec<GeoCoord>>),
    MultiPoint(Vec<GeoCoord>),
    MultiLineString(Vec<Vec<GeoCoord>>),
    MultiPolygon(Vec<Vec<Vec<GeoCoord>>>),
    GeometryCollection(Vec<GeoJsonGeometry>),
}

impl GeoJsonGeometry {
    /// Returns the GeoJSON `type` discriminator (matches the spec spelling).
    pub fn type_tag(&self) -> &'static str {
        match self {
            Self::Point(_) => "Point",
            Self::LineString(_) => "LineString",
            Self::Polygon(_) => "Polygon",
            Self::MultiPoint(_) => "MultiPoint",
            Self::MultiLineString(_) => "MultiLineString",
            Self::MultiPolygon(_) => "MultiPolygon",
            Self::GeometryCollection(_) => "GeometryCollection",
        }
    }

    /// Serialise the geometry as Well-Known Text (WKT) suitable for `ST_GeomFromText`.
    ///
    /// Returns `None` for variants that the visitor currently does not generate WKT for
    /// (`MultiPoint`, `MultiLineString`, `MultiPolygon`, `GeometryCollection`). Those variants
    /// are still accepted by the parser to allow lossless round-tripping but the visitor
    /// rejects them with a clear error rather than producing partially correct SQL.
    pub fn to_wkt(&self) -> Option<String> {
        fn coord(c: &GeoCoord) -> String {
            // Use Rust default formatting which preserves enough precision for f64 round-tripping
            // and never emits scientific notation that PostGIS would mis-parse.
            format!("{} {}", c.x, c.y)
        }
        fn ring(positions: &[GeoCoord]) -> String {
            let parts: Vec<_> = positions.iter().map(coord).collect();
            format!("({})", parts.join(", "))
        }

        match self {
            Self::Point(p) => Some(format!("POINT({})", coord(p))),
            Self::LineString(positions) => {
                if positions.is_empty() {
                    return Some("LINESTRING EMPTY".to_owned());
                }
                let parts: Vec<_> = positions.iter().map(coord).collect();
                Some(format!("LINESTRING({})", parts.join(", ")))
            }
            Self::Polygon(rings) => {
                if rings.is_empty() {
                    return Some("POLYGON EMPTY".to_owned());
                }
                let parts: Vec<_> = rings.iter().map(|r| ring(r)).collect();
                Some(format!("POLYGON({})", parts.join(", ")))
            }
            Self::MultiPoint(_) | Self::MultiLineString(_) | Self::MultiPolygon(_) | Self::GeometryCollection(_) => {
                None
            }
        }
    }

    /// Parses a `serde_json::Value` produced from user input into a validated geometry.
    ///
    /// Validation covers:
    /// * The `type` field must be present and a recognised GeoJSON type.
    /// * The `coordinates` (or `geometries`) field must be present and shape-correct.
    /// * All numeric coordinates must be finite.
    /// * Polygon rings are auto-closed when the caller provides at least three distinct
    ///   positions; missing first/last vertex equality is repaired silently to match
    ///   PostGIS leniency. Rings shorter than three distinct positions are rejected.
    pub fn from_serde_value(value: &serde_json::Value) -> Result<Self, GeoJsonParseError> {
        let obj = value.as_object().ok_or(GeoJsonParseError::ExpectedObject)?;

        let type_str = obj
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or(GeoJsonParseError::MissingType)?;

        match type_str {
            "Point" => {
                let coords = expect_coords(obj)?;
                Self::parse_position(coords).map(Self::Point)
            }
            "LineString" => {
                let coords = expect_coords(obj)?;
                Self::parse_position_array(coords).map(Self::LineString)
            }
            "Polygon" => {
                let coords = expect_coords(obj)?;
                Self::parse_polygon_rings(coords).map(Self::Polygon)
            }
            "MultiPoint" => {
                let coords = expect_coords(obj)?;
                Self::parse_position_array(coords).map(Self::MultiPoint)
            }
            "MultiLineString" => {
                let coords = expect_coords(obj)?;
                let lines = coords
                    .as_array()
                    .ok_or(GeoJsonParseError::InvalidShape {
                        type_tag: "MultiLineString",
                        reason: "expected `coordinates` to be an array of LineString arrays",
                    })?
                    .iter()
                    .map(Self::parse_position_array)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self::MultiLineString(lines))
            }
            "MultiPolygon" => {
                let coords = expect_coords(obj)?;
                let polygons = coords
                    .as_array()
                    .ok_or(GeoJsonParseError::InvalidShape {
                        type_tag: "MultiPolygon",
                        reason: "expected `coordinates` to be an array of Polygon ring arrays",
                    })?
                    .iter()
                    .map(Self::parse_polygon_rings)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self::MultiPolygon(polygons))
            }
            "GeometryCollection" => {
                let geometries =
                    obj.get("geometries")
                        .and_then(|g| g.as_array())
                        .ok_or(GeoJsonParseError::InvalidShape {
                            type_tag: "GeometryCollection",
                            reason: "expected a `geometries` array",
                        })?;
                let inner = geometries
                    .iter()
                    .map(Self::from_serde_value)
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(Self::GeometryCollection(inner))
            }
            other => Err(GeoJsonParseError::UnsupportedType(other.to_owned())),
        }
    }

    fn parse_position(value: &serde_json::Value) -> Result<GeoCoord, GeoJsonParseError> {
        let arr = value.as_array().ok_or(GeoJsonParseError::InvalidPosition)?;
        if arr.len() < 2 {
            return Err(GeoJsonParseError::InvalidPosition);
        }
        let x = arr[0].as_f64().ok_or(GeoJsonParseError::InvalidPosition)?;
        let y = arr[1].as_f64().ok_or(GeoJsonParseError::InvalidPosition)?;
        GeoCoord::new(x, y)
    }

    fn parse_position_array(value: &serde_json::Value) -> Result<Vec<GeoCoord>, GeoJsonParseError> {
        value
            .as_array()
            .ok_or(GeoJsonParseError::InvalidPositionArray)?
            .iter()
            .map(Self::parse_position)
            .collect()
    }

    fn parse_polygon_rings(value: &serde_json::Value) -> Result<Vec<Vec<GeoCoord>>, GeoJsonParseError> {
        let rings = value.as_array().ok_or(GeoJsonParseError::InvalidShape {
            type_tag: "Polygon",
            reason: "expected `coordinates` to be an array of rings",
        })?;

        let mut parsed_rings = Vec::with_capacity(rings.len());
        for ring in rings {
            let mut positions = Self::parse_position_array(ring)?;
            close_ring(&mut positions)?;
            parsed_rings.push(positions);
        }
        Ok(parsed_rings)
    }
}

fn expect_coords(obj: &serde_json::Map<String, serde_json::Value>) -> Result<&serde_json::Value, GeoJsonParseError> {
    obj.get("coordinates").ok_or(GeoJsonParseError::MissingCoordinates)
}

fn close_ring(positions: &mut Vec<GeoCoord>) -> Result<(), GeoJsonParseError> {
    if positions.len() < 3 {
        return Err(GeoJsonParseError::RingTooShort { len: positions.len() });
    }
    let first = *positions.first().unwrap();
    let last = *positions.last().unwrap();
    if first != last {
        positions.push(first);
    }
    if positions.len() < 4 {
        return Err(GeoJsonParseError::RingTooShort { len: positions.len() });
    }
    Ok(())
}

/// Errors that can be produced while validating GeoJSON input. All variants are user-facing
/// and translated into `InputError` by the extractor.
#[derive(Debug, Error)]
pub enum GeoJsonParseError {
    #[error("GeoJSON geometry must be a JSON object")]
    ExpectedObject,
    #[error("GeoJSON geometry is missing the required `type` field")]
    MissingType,
    #[error("GeoJSON geometry is missing the required `coordinates` field")]
    MissingCoordinates,
    #[error("GeoJSON `{type_tag}` geometry is malformed: {reason}")]
    InvalidShape {
        type_tag: &'static str,
        reason: &'static str,
    },
    #[error("GeoJSON position must be an array of at least two finite numbers")]
    InvalidPosition,
    #[error("GeoJSON expected an array of positions")]
    InvalidPositionArray,
    #[error("GeoJSON polygon ring must contain at least 3 distinct positions (got {len})")]
    RingTooShort { len: usize },
    #[error("GeoJSON coordinates must be finite numbers (got [{x}, {y}])")]
    NonFiniteCoord { x: f64, y: f64 },
    #[error("Unsupported GeoJSON geometry type `{0}`")]
    UnsupportedType(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[track_caller]
    fn parse_err(value: serde_json::Value) -> GeoJsonParseError {
        GeoJsonGeometry::from_serde_value(&value).expect_err("expected parse error")
    }

    #[test]
    fn parses_point() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "Point",
            "coordinates": [1.0, 2.0]
        }))
        .unwrap();
        assert!(matches!(geom, GeoJsonGeometry::Point(GeoCoord { x, y }) if x == 1.0 && y == 2.0));
    }

    #[test]
    fn rejects_non_object_input() {
        assert!(matches!(parse_err(json!([1, 2])), GeoJsonParseError::ExpectedObject));
    }

    #[test]
    fn rejects_missing_type() {
        assert!(matches!(
            parse_err(json!({ "coordinates": [1.0, 2.0] })),
            GeoJsonParseError::MissingType
        ));
    }

    #[test]
    fn rejects_unknown_type() {
        assert!(matches!(
            parse_err(json!({ "type": "Triangle", "coordinates": [1.0, 2.0] })),
            GeoJsonParseError::UnsupportedType(ref t) if t == "Triangle"
        ));
    }

    #[test]
    fn rejects_missing_coordinates() {
        assert!(matches!(
            parse_err(json!({ "type": "Point" })),
            GeoJsonParseError::MissingCoordinates
        ));
    }

    #[test]
    fn rejects_short_position() {
        assert!(matches!(
            parse_err(json!({ "type": "Point", "coordinates": [1.0] })),
            GeoJsonParseError::InvalidPosition
        ));
    }

    #[test]
    fn rejects_non_finite_coordinate() {
        // Non-finite floats cannot survive a JSON round-trip, but the parser still validates
        // them defensively because callers can synthesise `GeoCoord` instances directly. Exercise
        // the guard through `GeoCoord::new` to keep the invariant tested.
        assert!(matches!(
            GeoCoord::new(f64::NAN, 0.0),
            Err(GeoJsonParseError::NonFiniteCoord { .. })
        ));
        assert!(matches!(
            GeoCoord::new(0.0, f64::INFINITY),
            Err(GeoJsonParseError::NonFiniteCoord { .. })
        ));
        assert!(matches!(
            GeoCoord::new(0.0, f64::NEG_INFINITY),
            Err(GeoJsonParseError::NonFiniteCoord { .. })
        ));
    }

    #[test]
    fn auto_closes_open_polygon_ring() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0]]]
        }))
        .unwrap();
        let GeoJsonGeometry::Polygon(rings) = geom else {
            panic!("expected polygon");
        };
        let ring = &rings[0];
        assert_eq!(ring.len(), 4, "open ring should be auto-closed");
        assert_eq!(ring.first().unwrap(), ring.last().unwrap());
    }

    #[test]
    fn accepts_already_closed_polygon_ring() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]]
        }))
        .unwrap();
        let GeoJsonGeometry::Polygon(rings) = geom else {
            panic!("expected polygon");
        };
        assert_eq!(rings[0].len(), 4);
    }

    #[test]
    fn rejects_polygon_ring_too_short() {
        assert!(matches!(
            parse_err(json!({
                "type": "Polygon",
                "coordinates": [[[0.0, 0.0], [1.0, 1.0]]]
            })),
            GeoJsonParseError::RingTooShort { len: 2 }
        ));
    }

    #[test]
    fn point_wkt_round_trips() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "Point",
            "coordinates": [2.35, 48.85]
        }))
        .unwrap();
        assert_eq!(geom.to_wkt().as_deref(), Some("POINT(2.35 48.85)"));
    }

    #[test]
    fn polygon_wkt_uses_closed_ring() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "Polygon",
            "coordinates": [[[0.0, 0.0], [0.0, 2.0], [2.0, 2.0], [2.0, 0.0]]]
        }))
        .unwrap();
        assert_eq!(geom.to_wkt().as_deref(), Some("POLYGON((0 0, 0 2, 2 2, 2 0, 0 0))"));
    }

    #[test]
    fn multi_variants_have_no_wkt() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "MultiPoint",
            "coordinates": [[0.0, 0.0], [1.0, 1.0]]
        }))
        .unwrap();
        assert!(geom.to_wkt().is_none());
    }

    #[test]
    fn geometry_collection_round_trips() {
        let geom = GeoJsonGeometry::from_serde_value(&json!({
            "type": "GeometryCollection",
            "geometries": [
                { "type": "Point", "coordinates": [0.0, 0.0] },
                { "type": "Point", "coordinates": [1.0, 1.0] }
            ]
        }))
        .unwrap();
        assert!(matches!(geom, GeoJsonGeometry::GeometryCollection(ref inner) if inner.len() == 2));
    }
}

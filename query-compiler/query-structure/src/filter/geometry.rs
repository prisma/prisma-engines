use crate::*;

#[derive(Debug, Clone)]
pub struct GeometryFilter {
    pub field: ScalarFieldRef,
    pub condition: GeometryFilterCondition,
}

impl PartialEq for GeometryFilter {
    fn eq(&self, other: &Self) -> bool {
        self.field == other.field && self.condition == other.condition
    }
}

#[derive(Debug, Clone)]
pub enum GeometryFilterCondition {
    Near {
        point: GeoCoord,
        max_distance: f64,
        srid: Option<i32>,
    },
    Within {
        /// Polygon ring (closed: first vertex == last vertex, length ≥ 4) validated at
        /// extraction time.
        polygon: Vec<GeoCoord>,
        srid: Option<i32>,
    },
    Intersects {
        geometry: GeoJsonGeometry,
        srid: Option<i32>,
    },
}

impl PartialEq for GeometryFilterCondition {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                GeometryFilterCondition::Near {
                    point: p1,
                    max_distance: d1,
                    srid: s1,
                },
                GeometryFilterCondition::Near {
                    point: p2,
                    max_distance: d2,
                    srid: s2,
                },
            ) => p1 == p2 && d1.to_bits() == d2.to_bits() && s1 == s2,
            (
                GeometryFilterCondition::Within {
                    polygon: poly1,
                    srid: s1,
                },
                GeometryFilterCondition::Within {
                    polygon: poly2,
                    srid: s2,
                },
            ) => s1 == s2 && poly1 == poly2,
            (
                GeometryFilterCondition::Intersects { geometry: g1, srid: s1 },
                GeometryFilterCondition::Intersects { geometry: g2, srid: s2 },
            ) => s1 == s2 && g1 == g2,
            _ => false,
        }
    }
}

impl std::hash::Hash for GeometryFilter {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.field.hash(state);
        match &self.condition {
            GeometryFilterCondition::Near {
                point,
                max_distance,
                srid,
            } => {
                "Near".hash(state);
                point.hash(state);
                max_distance.to_bits().hash(state);
                srid.hash(state);
            }
            GeometryFilterCondition::Within { polygon, srid } => {
                "Within".hash(state);
                polygon.hash(state);
                srid.hash(state);
            }
            GeometryFilterCondition::Intersects { geometry, srid } => {
                "Intersects".hash(state);
                geometry.hash(state);
                srid.hash(state);
            }
        }
    }
}

impl Eq for GeometryFilter {}

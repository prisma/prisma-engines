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
        point: (f64, f64),
        max_distance: f64,
        srid: Option<i32>,
    },
    Within {
        polygon: Vec<(f64, f64)>,
        srid: Option<i32>,
    },
    Intersects {
        geometry: serde_json::Value,
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
            ) => {
                p1.0.to_bits() == p2.0.to_bits()
                    && p1.1.to_bits() == p2.1.to_bits()
                    && d1.to_bits() == d2.to_bits()
                    && s1 == s2
            }
            (
                GeometryFilterCondition::Within { polygon: poly1, srid: s1 },
                GeometryFilterCondition::Within { polygon: poly2, srid: s2 },
            ) => {
                s1 == s2
                    && poly1.len() == poly2.len()
                    && poly1.iter().zip(poly2.iter()).all(|((x1, y1), (x2, y2))| {
                        x1.to_bits() == x2.to_bits() && y1.to_bits() == y2.to_bits()
                    })
            }
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
            GeometryFilterCondition::Near { point, max_distance, srid } => {
                "Near".hash(state);
                point.0.to_bits().hash(state);
                point.1.to_bits().hash(state);
                max_distance.to_bits().hash(state);
                srid.hash(state);
            }
            GeometryFilterCondition::Within { polygon, srid } => {
                "Within".hash(state);
                for (x, y) in polygon {
                    x.to_bits().hash(state);
                    y.to_bits().hash(state);
                }
                srid.hash(state);
            }
            GeometryFilterCondition::Intersects { geometry, srid } => {
                "Intersects".hash(state);
                geometry.to_string().hash(state);
                srid.hash(state);
            }
        }
    }
}

impl Eq for GeometryFilter {}

use crate::*;

#[derive(Debug, Clone, PartialEq)]
pub struct GeometryFilter {
    pub field: ScalarFieldRef,
    pub condition: GeometryFilterCondition,
}

#[derive(Debug, Clone, PartialEq)]
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

use super::*;
use itertools::Itertools;

#[derive(Debug, Default)]
pub struct SqlDatamodelRenderer {}

impl SqlDatamodelRenderer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DatamodelRenderer for SqlDatamodelRenderer {
    fn render_id(&self, id: IdFragment) -> String {
        id.to_string()
    }

    fn render_m2m(&self, m2m: M2mFragment) -> String {
        format!(
            "{} {} {}",
            m2m.field_name,
            m2m.field_type,
            m2m.directives.iter().map(ToString::to_string).join(" ")
        )
    }
}

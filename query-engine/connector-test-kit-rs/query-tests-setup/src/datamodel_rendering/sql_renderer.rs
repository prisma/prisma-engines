use super::*;

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
}

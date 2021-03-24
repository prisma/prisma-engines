use super::*;

pub struct SqlSchemaRenderer {}

impl SchemaRenderer for SqlSchemaRenderer {
    fn render_id(&self, id: IdFragment) -> String {
        id.to_string()
    }
}

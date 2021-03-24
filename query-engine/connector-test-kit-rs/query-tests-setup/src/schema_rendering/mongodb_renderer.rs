use crate::Directive;

use super::*;

#[derive(Debug, Default)]
pub struct MongoDbSchemaRenderer {}

impl MongoDbSchemaRenderer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SchemaRenderer for MongoDbSchemaRenderer {
    fn render_id(&self, mut id: IdFragment) -> String {
        // Mongo IDs require an `_id` mapping.
        id.upsert_directive("map", |existing| match existing {
            Some(dir) => {
                dir.args = vec!["\"_id\"".to_owned()];
                None
            }
            None => Some(Directive::new("map", vec!["\"_id\""])),
        });

        id.to_string()
    }
}

#[cfg(test)]
mod mongo_render_tests {
    use super::*;
    use crate::IdFragment;

    #[test]
    fn add_id_mapping() {
        let fragment = IdFragment {
            field_name: "someIdField".to_owned(),
            field_type: "SomeType".to_owned(),
            directives: vec![Directive::new("id", vec![])],
        };

        let renderer = MongoDbSchemaRenderer::new();
        let rendered = renderer.render_id(fragment);

        assert_eq!(rendered, r#"someIdField SomeType @id @map("_id")"#)
    }

    #[test]
    fn update_id_mapping() {
        let fragment = IdFragment {
            field_name: "someIdField".to_owned(),
            field_type: "SomeType".to_owned(),
            directives: vec![Directive::new("id", vec![]), Directive::new("map", vec!["\"not_id\""])],
        };

        let renderer = MongoDbSchemaRenderer::new();
        let rendered = renderer.render_id(fragment);

        assert_eq!(rendered, r#"someIdField SomeType @id @map("_id")"#)
    }
}

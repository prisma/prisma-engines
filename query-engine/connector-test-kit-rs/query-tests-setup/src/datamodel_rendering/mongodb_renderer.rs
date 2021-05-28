use super::*;
use crate::Directive;

#[derive(Debug, Default)]
pub struct MongoDbSchemaRenderer {}

impl MongoDbSchemaRenderer {
    pub fn new() -> Self {
        Self::default()
    }
}

impl DatamodelRenderer for MongoDbSchemaRenderer {
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

    // Currently just an accepted hack for MongoDB
    fn render_m2m(&self, m2m: M2mFragment) -> String {
        // Add an array field for mongo, name: "<rel_field_name>_ids <Opposing type>[]"
        let fk_field_name = format!("{}_ids", m2m.field_name);
        let additional_fk_field = format!("{} {}[]", fk_field_name, m2m.opposing_type);

        // Add @relation directive that specifies the local array to hold the FKs.
        let relation_directive = match m2m.relation_name {
            Some(name) => format!(r#"@relation(name: "{}", fields: [{}])"#, name, fk_field_name),
            None => format!(r#"@relation(fields: [{}])"#, fk_field_name),
        };

        format!(
            "{}\n{} {} {}",
            additional_fk_field, m2m.field_name, m2m.field_type, relation_directive,
        )
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

    #[test]
    fn add_m2m_mapping() {
        let fragment = M2mFragment {
            field_name: "posts".to_owned(),
            field_type: "Post[]".to_owned(),
            opposing_type: "String".to_owned(),
            relation_name: Some("test".to_owned()),
        };

        let renderer = MongoDbSchemaRenderer::new();
        let rendered = renderer.render_m2m(fragment);

        assert_eq!(
            rendered.trim(),
            "posts_ids String[]\nposts Post[] @relation(name: \"test\", fields: [posts_ids])"
        )
    }
}

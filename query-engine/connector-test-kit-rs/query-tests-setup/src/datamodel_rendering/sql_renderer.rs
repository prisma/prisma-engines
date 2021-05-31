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

    fn render_m2m(&self, m2m: M2mFragment) -> String {
        let relation_directive = match m2m.relation_name {
            Some(name) => format!(r#"@relation(name: "{}")"#, name),
            None => "".to_owned(),
        };

        format!("{} {} {}", m2m.field_name, m2m.field_type, relation_directive)
    }
}

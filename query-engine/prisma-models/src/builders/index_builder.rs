use crate::{Index, IndexType, ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug)]
pub struct IndexBuilder {
    pub name: Option<String>,
    pub fields: Vec<String>,
    pub typ: IndexType,
}

impl IndexBuilder {
    pub fn build(self, fields: &[ScalarFieldRef]) -> Index {
        let fields = match self.typ {
            IndexType::Unique => Self::map_fields(self.fields, fields),
            IndexType::Normal => Self::map_fields(self.fields, fields),
        };

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
    }

    fn map_fields(field_names: Vec<String>, fields: &[ScalarFieldRef]) -> Vec<ScalarFieldWeak> {
        field_names
            .into_iter()
            .map(|name| {
                fields
                    .iter()
                    .find(|sf| sf.name() == name)
                    .unwrap_or_else(|| panic!("Unable to resolve field '{name}'"))
                    .clone()
            })
            .collect()
    }
}

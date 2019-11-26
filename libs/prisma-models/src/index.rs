use crate::{ScalarFieldRef, ScalarFieldWeak};
use std::sync::Arc;

#[derive(Debug)]
pub struct IndexTemplate {
    pub name: Option<String>,
    pub fields: Vec<String>,
    pub typ: IndexType,
}

impl IndexTemplate {
    pub fn build(self, fields: &[ScalarFieldRef]) -> Index {
        let fields = self
            .fields
            .into_iter()
            .map(|name| {
                let field = fields.iter().find(|sf| sf.name == name).unwrap();
                Arc::downgrade(field)
            })
            .collect();

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
    }
}

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<ScalarFieldWeak>,
    pub typ: IndexType,
}

#[derive(Debug, Copy, Clone)]
pub enum IndexType {
    Unique,
    Normal,
}

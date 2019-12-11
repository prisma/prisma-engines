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
        let fields = match self.typ {
            IndexType::Unique => self.resolve_fields(fields),
            IndexType::Normal => vec![],
        };

        Index {
            name: self.name,
            typ: self.typ,
            fields,
        }
    }

    fn resolve_fields(self, fields: &[ScalarFieldRef]) -> Vec<ScalarFieldWeak> {
        self.fields
            .into_iter()
            .map(|name| {
                let field = fields
                    .iter()
                    .find(|sf| sf.name == name)
                    .expect(&format!("Unable to resolve scalar field '{}'", name));

                Arc::downgrade(field)
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct Index {
    pub name: Option<String>,
    pub fields: Vec<ScalarFieldWeak>,
    pub typ: IndexType,
}

impl Index {
    pub fn fields(&self) -> Vec<ScalarFieldRef> {
        self.fields.iter().map(|sf| sf.upgrade().unwrap()).collect()
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum IndexType {
    Unique,
    Normal,
}

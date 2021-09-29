use std::sync::Arc;

use crate::{Field, ScalarFieldRef, ScalarFieldWeak};

#[derive(Debug)]
pub struct PrimaryKeyTemplate {
    pub alias: Option<String>,
    pub fields: Vec<String>,
}

impl PrimaryKeyTemplate {
    pub fn build(self, all_fields: &[Field]) -> PrimaryKey {
        PrimaryKey {
            fields: self
                .fields
                .iter()
                .map(|f| {
                    let f = all_fields.iter().find(|field| field.name() == f).unwrap().clone();

                    Arc::downgrade(&f.try_into_scalar().unwrap())
                })
                .collect::<Vec<_>>(),
            alias: self.alias,
        }
    }
}

#[derive(Debug)]
pub struct PrimaryKey {
    pub alias: Option<String>,
    fields: Vec<ScalarFieldWeak>,
}

impl PrimaryKey {
    pub fn fields(&self) -> Vec<ScalarFieldRef> {
        self.fields.iter().map(|field| field.upgrade().unwrap()).collect()
    }
}

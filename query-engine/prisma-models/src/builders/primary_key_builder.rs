use crate::{pk::PrimaryKey, Field};
use std::sync::Arc;

#[derive(Debug)]
pub struct PrimaryKeyBuilder {
    pub alias: Option<String>,
    pub fields: Vec<String>,
}

impl PrimaryKeyBuilder {
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

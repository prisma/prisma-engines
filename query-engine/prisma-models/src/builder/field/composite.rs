use crate::{CompositeField, CompositeFieldRef, CompositeTypeRef, ModelWeakRef};
use datamodel::FieldArity;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct CompositeFieldBuilder {
    pub name: String,
    pub db_name: Option<String>,
    pub is_required: bool,
    pub arity: FieldArity,
    pub type_name: String,
}

impl CompositeFieldBuilder {
    pub fn build(self, model: ModelWeakRef, composite_types: &[CompositeTypeRef]) -> CompositeFieldRef {
        let composite = CompositeField {
            name: self.name,
            db_name: self.db_name,
            typ: composite_types
                .into_iter()
                .find(|typ| &typ.name == &self.type_name)
                .expect(&format!("Invalid composite type reference: {}", self.type_name))
                .clone(),
            arity: self.arity,
            model: model,
        };

        Arc::new(composite)
    }
}

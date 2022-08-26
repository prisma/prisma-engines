use crate::{
    builders::IndexBuilder, parent_container::ParentContainer, CompositeField, CompositeFieldRef, CompositeTypeRef,
    Field,
};
use datamodel::dml::{DefaultValue, FieldArity};
use once_cell::sync::OnceCell;
use std::{fmt::Debug, sync::Arc};

#[derive(Debug)]
pub struct CompositeFieldBuilder {
    pub name: String,
    pub db_name: Option<String>,
    pub arity: FieldArity,
    pub type_name: String,
    pub default_value: Option<DefaultValue>,
    pub unique_index: Option<IndexBuilder>,
}

impl CompositeFieldBuilder {
    pub fn build(self, container: ParentContainer, composite_types: &[CompositeTypeRef]) -> CompositeFieldRef {
        let type_name = &self.type_name;
        let composite = CompositeField {
            name: self.name,
            db_name: self.db_name,
            typ: composite_types
                .iter()
                .find(|typ| &typ.name == type_name)
                .unwrap_or_else(|| panic!("Invalid composite type reference: {}", type_name))
                .clone(),
            arity: self.arity,
            container,
            unique_index: OnceCell::new(),
        };
        let composite = Arc::new(composite);

        composite
            .unique_index
            .set(
                self.unique_index
                    .map(|u| u.build(&[Field::Composite(composite.clone())])),
            )
            .unwrap();

        composite
    }
}

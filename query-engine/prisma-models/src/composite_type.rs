use crate::{Field, InternalDataModelRef, InternalDataModelWeakRef};
use once_cell::sync::OnceCell;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type CompositeTypeRef = Arc<CompositeType>;
pub type CompositeTypeWeakRef = Weak<CompositeType>;

// #[derive(Debug)]
// pub struct CompositeTypeTemplate {
//     pub name: String,
//     pub fields: Vec<FieldTemplate>,
//     // pub dml_model: datamodel::Model,
// }

// impl CompositeTypeTemplate {
//     pub fn build(self) -> CompositeTypeRef {
//         todo!()
//     }
// }

#[derive(Debug)]
pub struct CompositeType {
    /// Then name of the composite type.
    /// Unique across all models, enums, composite types.
    pub name: String,

    /// Back-reference to the internal data model.
    pub internal_data_model: InternalDataModelWeakRef,

    /// Fields of this composite type.
    /// May contain other composites and even cycles.
    pub(crate) fields: OnceCell<Vec<Field>>,
}

impl CompositeType {
    pub fn internal_data_model(&self) -> InternalDataModelRef {
        self.internal_data_model
            .upgrade()
            .expect("Invalid back-reference to internal data model.")
    }
}

impl Hash for CompositeType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Names are unique in the data model.
        self.name.hash(state);
    }
}

impl PartialEq for CompositeType {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

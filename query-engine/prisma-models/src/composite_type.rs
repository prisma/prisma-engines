use crate::Fields;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Weak};

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

    /// Fields of this composite type.
    /// May contain other composites and even cycles.
    fields: OnceCell<Fields>,
}

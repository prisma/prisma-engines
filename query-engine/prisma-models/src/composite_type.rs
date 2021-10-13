use crate::{FieldTemplate, Fields, InternalDataModelWeakRef};
use once_cell::sync::OnceCell;
use std::sync::{Arc, Weak};

pub type CompositeTypeRef = Arc<CompositeType>;
pub type CompositeTypeWeakRef = Weak<CompositeType>;

#[derive(Debug)]
pub struct CompositeTypeTemplate {
    pub name: String,
    pub fields: Vec<FieldTemplate>,
    // pub dml_model: datamodel::Model,
}

#[derive(Debug)]
pub struct CompositeType {
    pub name: String,
    fields: OnceCell<Fields>,
    // dml_model: datamodel::Model,
    pub internal_data_model: InternalDataModelWeakRef,
}

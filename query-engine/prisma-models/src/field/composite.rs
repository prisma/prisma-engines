use crate::{parent_container::ParentContainer, CompositeTypeRef, ScalarFieldRef};
use datamodel::FieldArity;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type CompositeFieldRef = Arc<CompositeField>;
pub type CompositeFieldWeak = Weak<CompositeField>;

#[derive(Clone)]
pub struct CompositeField {
    pub name: String,
    pub typ: CompositeTypeRef,
    pub(crate) db_name: Option<String>,
    pub(crate) arity: FieldArity,
    pub(crate) container: ParentContainer,
}

impl CompositeField {
    pub fn is_list(&self) -> bool {
        matches!(self.arity, FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity, FieldArity::Required)
    }

    pub fn db_name(&self) -> &str {
        self.db_name.as_deref().unwrap_or_else(|| self.name.as_str())
    }

    pub fn scalar_fields(&self) -> Vec<ScalarFieldRef> {
        // let fields = self.fields.get_or_init(|| {
        //     let model = self.model();
        //     let fields = model.fields();

        //     self.relation_info
        //         .fields
        //         .iter()
        //         .map(|f| {
        //             Arc::downgrade(&fields.find_from_scalar(f).unwrap_or_else(|_| {
        //                 panic!(
        //                     "Expected '{}' to be a scalar field on model '{}', found none.",
        //                     f, model.name
        //                 )
        //             }))
        //         })
        //         .collect()
        // });

        // fields.iter().map(|f| f.upgrade().unwrap()).collect()
        todo!()
    }

    pub fn container(&self) -> &ParentContainer {
        &self.container
    }
}

impl Debug for CompositeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeField")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .field("container", &self.container)
            .field("composite_type", &self.typ.name)
            .finish()
    }
}

impl Hash for CompositeField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Names are unique in the data model.
        self.name.hash(state);
    }
}

impl Eq for CompositeField {}

impl PartialEq for CompositeField {
    fn eq(&self, other: &CompositeField) -> bool {
        self.name == other.name
    }
}

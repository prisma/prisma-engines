use crate::{Fields, ModelRef, ModelWeakRef, ScalarFieldRef};
use datamodel::FieldArity;
use once_cell::sync::OnceCell;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type CompositeFieldRef = Arc<CompositeField>;
pub type CompositeFieldWeak = Weak<CompositeField>;

#[derive(Debug)]
pub struct CompositeFieldTemplate {
    pub name: String,
    pub is_required: bool,
    pub arity: FieldArity,
    // typ:
}

impl CompositeFieldTemplate {
    pub fn build(self, _model: ModelWeakRef) -> CompositeFieldRef {
        // let scalar = ScalarField {
        //     name: self.name,
        //     type_identifier: self.type_identifier,
        //     is_id: self.is_id,
        //     is_required: self.is_required,
        //     is_list: self.is_list,
        //     is_autoincrement: self.is_autoincrement,
        //     is_auto_generated_int_id: self.is_auto_generated_int_id,
        //     read_only: OnceCell::new(),
        //     is_unique: self.is_unique,
        //     internal_enum: self.internal_enum,
        //     behaviour: self.behaviour,
        //     arity: self.arity,
        //     db_name: self.db_name,
        //     default_value: self.default_value,
        //     native_type: self.native_type,
        //     model,
        // };

        // Arc::new(scalar)

        todo!()
    }
}

#[derive(Clone)]
pub struct CompositeField {
    pub name: String,
    pub arity: FieldArity,
    pub model: ModelWeakRef,

    fields: OnceCell<Fields>,
}

impl CompositeField {
    pub fn is_list(&self) -> bool {
        matches!(self.arity, FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity, FieldArity::Required)
    }

    pub fn model(&self) -> ModelRef {
        self.model
            .upgrade()
            .expect("Model does not exist anymore. Parent model got deleted without deleting the child.")
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
}

impl Debug for CompositeField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeField")
            .field("name", &self.name)
            .field("arity", &self.arity)
            .field("model", &"#ModelWeakRef#")
            .field("fields", &self.fields)
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

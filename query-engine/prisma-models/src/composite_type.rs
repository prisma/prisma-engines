use crate::{CompositeFieldId, Field, InternalDataModelRef, InternalDataModelWeakRef};
use once_cell::sync::OnceCell;
use psl::schema_ast::ast;
use std::{
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type CompositeTypeRef = Arc<CompositeType>;
pub type CompositeTypeWeakRef = Weak<CompositeType>;

#[derive(Debug)]
pub struct CompositeType {
    pub id: ast::CompositeTypeId,

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

    pub fn fields(&self) -> impl Iterator<Item = Field> + '_ {
        let internal_data_model = self.internal_data_model();
        let cf: Vec<_> = internal_data_model
            .walk(self.id)
            .fields()
            .filter(|f| f.r#type().as_composite_type().is_some())
            .map(|f| Field::from(internal_data_model.clone().zip(CompositeFieldId::InCompositeType(f.id))))
            .collect();
        self.fields.get().unwrap().iter().cloned().chain(cf)
    }

    pub fn find_field(&self, prisma_name: &str) -> Option<Field> {
        self.fields().find(|f| f.name() == prisma_name)
    }

    pub fn find_field_by_db_name(&self, db_name: &str) -> Option<Field> {
        self.fields().find(|f| f.db_name() == db_name)
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

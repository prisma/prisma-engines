use crate::{composite_type::CompositeType, model::Model};
use psl_core::schema_ast::ast;

#[derive(Debug, Default)]
pub struct Datamodel {
    pub models: Vec<Model>,
    pub composite_types: Vec<CompositeType>,
}

impl Datamodel {
    /// Gets an iterator over all models.
    pub fn models(&self) -> std::slice::Iter<Model> {
        self.models.iter()
    }

    /// Gets an iterator over all composite types.
    pub fn composite_types(&self) -> std::slice::Iter<CompositeType> {
        self.composite_types.iter()
    }

    /// Finds a model by name.
    pub fn find_model(&self, name: &str) -> Option<&Model> {
        self.models().find(|model| model.name == name)
    }

    pub fn find_model_by_id(&self, id: ast::ModelId) -> Option<&Model> {
        self.models().find(|m| m.id == id)
    }

    /// Finds a composite type by name.
    pub fn find_composite_type(&self, name: &str) -> Option<&CompositeType> {
        self.composite_types().find(|composite| composite.name == name)
    }

    /// Finds a model by database name. This will only find models with a name
    /// remapped to the provided `db_name`.
    pub fn find_model_db_name(&self, db_name: &str) -> Option<&Model> {
        self.models()
            .find(|model| model.database_name.as_deref() == Some(db_name))
    }

    /// Finds a model by name and returns a mutable reference.
    pub fn find_model_mut(&mut self, name: &str) -> &mut Model {
        self.models
            .iter_mut()
            .find(|m| m.name == *name)
            .expect("We assume an internally valid datamodel before mutating.")
    }
}

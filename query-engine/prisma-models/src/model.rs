use crate::prelude::*;
use once_cell::sync::OnceCell;
use psl::schema_ast::ast;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type ModelRef = Arc<Model>;
pub type ModelWeakRef = Weak<Model>;

pub struct Model {
    pub id: ast::ModelId,
    pub(crate) name: String,
    pub(crate) manifestation: Option<String>,
    pub(crate) fields: OnceCell<Fields>,
    pub(crate) indexes: OnceCell<Vec<Index>>,
    pub(crate) primary_identifier: OnceCell<FieldSelection>,
    pub(crate) dml_model: dml::Model,

    pub internal_data_model: InternalDataModelWeakRef,
}

impl Model {
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the schema name for the model
    /// which is the contents of the @@schema("...") attribute
    pub fn schema_name(&self) -> Option<String> {
        self.dml_model.schema.clone()
    }

    /// Returns the set of fields to be used as the primary identifier for a record of that model.
    /// The identifier is nothing but an internal convention to have an anchor point for querying, or in other words,
    /// the identifier is not to be mistaken for a stable, external identifier, but has to be understood as
    /// implementation detail that is used to reason over a fixed set of fields.
    pub fn primary_identifier(&self) -> FieldSelection {
        self.primary_identifier.get_or_init(||{
            let dml_fields = self.dml_model.first_unique_criterion();
            let fields: Vec<_> = dml_fields
                .iter()
                .map(|dml_field| {
                    self.fields().find_from_scalar(&dml_field.name).unwrap_or_else(|_| panic!("Error finding primary identifier: The parser field {} does not exist in the query engine datamodel.", &dml_field.name))
                })
            .collect();

            FieldSelection::from(fields)
        }).clone()
    }

    pub fn fields(&self) -> &Fields {
        self.fields
            .get()
            .ok_or_else(|| String::from("Model fields must be set!"))
            .unwrap()
    }

    pub fn supports_create_operation(&self) -> bool {
        let dm = self.internal_data_model();
        let walker = dm.walk(self.id);
        let has_unsupported_field = walker
            .scalar_fields()
            .any(|sf| sf.ast_field().arity.is_required() && sf.is_unsupported() && sf.default_value().is_none());

        !has_unsupported_field
    }

    pub fn indexes(&self) -> &[Index] {
        self.indexes
            .get()
            .ok_or_else(|| String::from("Model indexes must be set!"))
            .unwrap()
    }

    pub fn unique_indexes(&self) -> Vec<&Index> {
        self.indexes()
            .iter()
            .filter(|index| index.typ == IndexType::Unique)
            .collect()
    }

    /// The name of the model in the database
    /// For a sql database this will be the Table name for this model
    pub fn db_name(&self) -> &str {
        self.db_name_opt().unwrap_or_else(|| self.name.as_ref())
    }

    pub fn db_name_opt(&self) -> Option<&str> {
        self.manifestation.as_ref().map(|m| m.as_ref())
    }

    pub fn internal_data_model(&self) -> InternalDataModelRef {
        self.internal_data_model
            .upgrade()
            .expect("InternalDataModel does not exist anymore. Parent internal_data_model is deleted without deleting the child internal_data_model.")
    }
}

impl Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("name", &self.name)
            .field("schema", &self.schema_name().unwrap_or_default())
            .field("manifestation", &self.manifestation)
            .field("fields", &self.fields)
            .field("indexes", &self.indexes)
            .field("primary_identifier", &self.primary_identifier)
            .field("dml_model", &self.dml_model)
            .field("internal_data_model", &"#InternalDataModelWeakRef#")
            .finish()
    }
}

impl Hash for Model {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Names are unique in the data model.
        self.name.hash(state);
    }
}

impl Eq for Model {}

impl PartialEq for Model {
    fn eq(&self, other: &Model) -> bool {
        self.name == other.name
    }
}

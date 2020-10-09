use crate::prelude::*;
use once_cell::sync::OnceCell;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type ModelRef = Arc<Model>;
pub type ModelWeakRef = Weak<Model>;

#[derive(Debug)]
pub struct ModelTemplate {
    pub name: String,
    pub is_embedded: bool,
    pub fields: Vec<FieldTemplate>,
    pub manifestation: Option<String>,
    pub id_field_names: Vec<String>,
    pub indexes: Vec<IndexTemplate>,
    pub dml_model: datamodel::Model,
}

pub struct Model {
    pub name: String,
    pub is_embedded: bool,

    manifestation: Option<String>,
    fields: OnceCell<Fields>,
    indexes: OnceCell<Vec<Index>>,
    primary_identifier: OnceCell<ModelProjection>,
    dml_model: datamodel::Model,

    pub internal_data_model: InternalDataModelWeakRef,
}

impl Debug for Model {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Model")
            .field("name", &self.name)
            .field("is_embedded", &self.is_embedded)
            .field("manifestation", &self.manifestation)
            .field("fields", &self.fields)
            .field("indexes", &self.indexes)
            .field("primary_identifier", &self.primary_identifier)
            .field("dml_model", &self.dml_model)
            .field("internal_data_model", &"#InternalDataModelWeakRef#")
            .finish()
    }
}

impl ModelTemplate {
    pub fn build(self, internal_data_model: InternalDataModelWeakRef) -> ModelRef {
        let model = Arc::new(Model {
            name: self.name,
            is_embedded: self.is_embedded,
            manifestation: self.manifestation,
            fields: OnceCell::new(),
            indexes: OnceCell::new(),
            primary_identifier: OnceCell::new(),
            dml_model: self.dml_model,
            internal_data_model,
        });

        let fields = Fields::new(
            self.fields
                .into_iter()
                .map(|ft| ft.build(Arc::downgrade(&model)))
                .collect(),
            Arc::downgrade(&model),
            self.id_field_names,
        );

        let indexes = self.indexes.into_iter().map(|i| i.build(&fields.scalar())).collect();

        // The model is created here and fields WILL BE UNSET before now!
        model.fields.set(fields).unwrap();
        model.indexes.set(indexes).unwrap();
        model
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

impl Model {
    pub(crate) fn finalize(&self) {
        self.fields.get().unwrap().finalize();
    }

    /// Returns the set of fields to be used as the primary identifier for a record of that model.
    /// The identifier is nothing but an internal convention to have an anchor point for querying, or in other words,
    /// the identifier is not to be mistaken for a stable, external identifier, but has to be understood as
    /// implementation detail that is used to reason over a fixed set of fields.
    pub fn primary_identifier(&self) -> ModelProjection {
        self.primary_identifier.get_or_init(||{
            let dml_fields = self.dml_model.first_unique_criterion();
            let fields: Vec<_> = dml_fields
                .iter()
                .map(|dml_field| {
                    let field = self.fields().find_from_all(&dml_field.name).unwrap_or_else(|_| panic!("Error finding primary identifier: The parser field {} does not exist in the query engine datamodel.", &dml_field.name));
                    field.clone()
                })
                .collect();

            ModelProjection::new(fields)
        }).clone()
    }

    pub fn fields(&self) -> &Fields {
        self.fields
            .get()
            .ok_or_else(|| String::from("Model fields must be set!"))
            .unwrap()
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

    pub fn is_legacy(&self) -> bool {
        self.internal_data_model().is_legacy()
    }

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

    pub fn map_scalar_db_field_name(&self, name: &str) -> Option<ScalarFieldRef> {
        self.fields()
            .scalar()
            .into_iter()
            .find_map(|field| if field.db_name() == name { Some(field) } else { None })
    }
}

use crate::prelude::*;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Weak};

pub type InternalDataModelRef = Arc<InternalDataModel>;
pub type InternalDataModelWeakRef = Weak<InternalDataModel>;
pub type InternalEnumRef = Arc<InternalEnum>;

#[derive(Debug, Default)]
pub struct InternalDataModelTemplate {
    pub models: Vec<ModelTemplate>,
    pub relations: Vec<RelationTemplate>,
    pub enums: Vec<InternalEnum>,
    pub version: Option<String>,
}

#[derive(Debug)]
pub struct InternalDataModel {
    pub enums: Vec<InternalEnumRef>,
    version: Option<String>,

    /// Todo clarify / rename.
    /// The db name influences how data is queried from the database.
    /// E.g. this influences the schema part of a postgres query: `database`.`schema`.`table`.
    /// Other connectors do not use `schema`, like postgres does, and this variable would
    /// influence the `database` part instead.
    pub db_name: String,

    models: OnceCell<Vec<ModelRef>>,
    relations: OnceCell<Vec<RelationRef>>,
    relation_fields: OnceCell<Vec<RelationFieldRef>>,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct InternalEnum {
    pub name: String,
    pub values: Vec<InternalEnumValue>,
}

impl InternalEnum {
    pub fn new<N, I, V>(name: N, values: I) -> Self
    where
        N: Into<String>,
        V: Into<InternalEnumValue>,
        I: IntoIterator<Item = V>,
    {
        Self {
            name: name.into(),
            values: values.into_iter().map(|v| v.into()).collect(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct InternalEnumValue {
    pub name: String,
    pub database_name: Option<String>,
}

impl InternalEnumValue {
    pub fn new<N, I, V>(name: N, database_name: I) -> Self
    where
        N: Into<String>,
        V: Into<String>,
        I: Into<Option<String>>,
    {
        Self {
            name: name.into(),
            database_name: database_name.into(),
        }
    }

    pub fn db_name(&self) -> &String {
        self.database_name.as_ref().unwrap_or(&self.name)
    }
}

impl InternalDataModelTemplate {
    pub fn build(self, db_name: String) -> InternalDataModelRef {
        let internal_data_model = Arc::new(InternalDataModel {
            models: OnceCell::new(),
            relations: OnceCell::new(),
            enums: self.enums.into_iter().map(Arc::new).collect(),
            version: self.version,
            db_name,
            relation_fields: OnceCell::new(),
        });

        let models = self
            .models
            .into_iter()
            .map(|mt| mt.build(Arc::downgrade(&internal_data_model)))
            .collect();

        internal_data_model.models.set(models).unwrap();

        let relations = self
            .relations
            .into_iter()
            .map(|rt| rt.build(Arc::downgrade(&internal_data_model)))
            .collect();

        internal_data_model.relations.set(relations).unwrap();
        internal_data_model.finalize();
        internal_data_model
    }
}

impl InternalDataModel {
    fn finalize(&self) {
        self.models().iter().for_each(|model| model.finalize());
    }

    pub fn models(&self) -> &[ModelRef] {
        self.models.get().unwrap()
    }

    pub fn relations(&self) -> &[RelationRef] {
        self.relations.get().unwrap().as_slice()
    }

    pub fn find_enum(&self, name: &str) -> crate::Result<InternalEnumRef> {
        self.enums
            .iter()
            .find(|e| e.name == name)
            .cloned()
            .ok_or_else(|| DomainError::EnumNotFound { name: name.to_string() })
    }

    pub fn find_model(&self, name: &str) -> crate::Result<ModelRef> {
        self.models
            .get()
            .and_then(|models| models.iter().find(|model| model.name == name))
            .cloned()
            .ok_or_else(|| DomainError::ModelNotFound { name: name.to_string() })
    }

    pub fn find_relation(&self, name: &str) -> crate::Result<RelationWeakRef> {
        self.relations
            .get()
            .and_then(|relations| relations.iter().find(|relation| relation.name == name))
            .map(|relation| Arc::downgrade(relation))
            .ok_or_else(|| DomainError::RelationNotFound { name: name.to_string() })
    }

    pub fn is_legacy(&self) -> bool {
        self.version.is_none()
    }

    /// Finds all non-list relation fields pointing to the given model.
    /// `required` may narrow down the returned fields to required fields only. Returns all on `false`.
    pub fn fields_pointing_to_model(&self, model: &ModelRef, required: bool) -> Vec<RelationFieldRef> {
        self.relation_fields()
            .iter()
            .filter(|rf| &rf.related_model() == model) // All relation fields pointing to `model`.
            .filter(|rf| rf.is_inlined_on_enclosing_model()) // Not a list, not a virtual field.
            .filter(|rf| !required || rf.is_required) // If only required fields should be returned
            .map(Arc::clone)
            .collect()
    }

    /// Finds all relation fields where the foreign key refers to the given field (as either singular or compound).
    pub fn fields_refering_to_field(&self, field: &ScalarFieldRef) -> Vec<RelationFieldRef> {
        let model_name = &field.model().name;

        self.relation_fields()
            .iter()
            .filter(|rf| &rf.relation_info.to == model_name)
            .filter(|rf| rf.relation_info.references.contains(&field.name))
            .map(Arc::clone)
            .collect()
    }

    pub fn relation_fields(&self) -> &[RelationFieldRef] {
        self.relation_fields
            .get_or_init(|| {
                self.models()
                    .iter()
                    .flat_map(|model| model.fields().relation())
                    .collect()
            })
            .as_slice()
    }

    pub fn non_embedded_models(&self) -> Vec<ModelRef> {
        self.models()
            .iter()
            .filter(|m| !m.is_embedded)
            .map(|m| Arc::clone(m))
            .collect()
    }
}

use crate::prelude::*;
use once_cell::sync::OnceCell;
use std::sync::{Arc, Weak};

pub type InternalDataModelRef = Arc<InternalDataModel>;
pub type InternalDataModelWeakRef = Weak<InternalDataModel>;

#[derive(Debug, Default)]
pub struct InternalDataModelTemplate {
    pub models: Vec<ModelTemplate>,
    pub relations: Vec<RelationTemplate>,
    pub enums: Vec<InternalEnum>,
    pub version: Option<String>,
}

#[derive(DebugStub)]
pub struct InternalDataModel {
    pub enums: Vec<InternalEnum>,
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

    pub fn map_input_value(&self, val: &String) -> Option<PrismaValue> {
        Some(PrismaValue::Enum(
            self.values.iter().find(|ev| &ev.name == val)?.final_db_name().clone(),
        ))
    }

    pub fn map_output_value(&self, val: &String) -> Option<PrismaValue> {
        Some(PrismaValue::Enum(
            self.values.iter().find(|ev| ev.final_db_name() == val)?.name.clone(),
        ))
    }

    pub fn external_values(&self) -> Vec<String> {
        self.values.iter().map(|v| v.name.to_string()).collect::<Vec<String>>()
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

    pub fn final_db_name(&self) -> &String {
        self.database_name.as_ref().unwrap_or(&self.name)
    }
}

impl InternalDataModelTemplate {
    pub fn build(self, db_name: String) -> InternalDataModelRef {
        let internal_data_model = Arc::new(InternalDataModel {
            models: OnceCell::new(),
            relations: OnceCell::new(),
            enums: self.enums,
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
        internal_data_model
    }
}

impl InternalDataModel {
    pub fn models(&self) -> &[ModelRef] {
        self.models.get().unwrap()
    }

    pub fn relations(&self) -> &[RelationRef] {
        self.relations.get().unwrap().as_slice()
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
            .map(|relation| Arc::downgrade(&relation))
            .ok_or_else(|| DomainError::RelationNotFound { name: name.to_string() })
    }

    pub fn is_legacy(&self) -> bool {
        self.version.is_none()
    }

    pub fn fields_requiring_model(&self, model: &ModelRef) -> Vec<RelationFieldRef> {
        self.relation_fields()
            .iter()
            .filter(|rf| &rf.related_model() == model)
            .filter(|f| f.is_required && !f.is_list)
            .map(|f| Arc::clone(f))
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
}

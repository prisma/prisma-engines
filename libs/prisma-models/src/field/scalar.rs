use crate::prelude::*;
use datamodel::{DefaultValue, FieldArity};
use once_cell::sync::OnceCell;
use std::{
    fmt::Debug,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

static ID_FIELD: &str = "id";
static EMBEDDED_ID_FIELD: &str = "_id";
static UPDATED_AT_FIELD: &str = "updatedAt";
static CREATED_AT_FIELD: &str = "createdAt";

pub type ScalarFieldRef = Arc<ScalarField>;
pub type ScalarFieldWeak = Weak<ScalarField>;

#[derive(Debug)]
pub struct ScalarFieldTemplate {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_unique: bool,
    pub is_id: bool,
    pub is_auto_generated_int_id: bool,
    pub is_autoincrement: bool,
    pub behaviour: Option<FieldBehaviour>,
    pub internal_enum: Option<InternalEnum>,
    pub arity: FieldArity,
    pub db_name: Option<String>,
    pub default_value: Option<DefaultValue>,
}

pub struct ScalarField {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_id: bool,
    pub is_auto_generated_int_id: bool,
    pub is_autoincrement: bool,
    pub internal_enum: Option<InternalEnum>,
    pub behaviour: Option<FieldBehaviour>,
    pub arity: FieldArity,
    pub db_name: Option<String>,
    pub default_value: Option<DefaultValue>,

    pub model: ModelWeakRef,
    pub(crate) is_unique: bool,
    pub(crate) read_only: OnceCell<bool>,
}

impl Debug for ScalarField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarField")
            .field("name", &self.name)
            .field("type_identifier", &self.type_identifier)
            .field("is_required", &self.is_required)
            .field("is_list", &self.is_list)
            .field("is_id", &self.is_id)
            .field("is_auto_generated_int_id", &self.is_auto_generated_int_id)
            .field("is_autoincrement", &self.is_autoincrement)
            .field("internal_enum", &self.internal_enum)
            .field("behaviour", &self.behaviour)
            .field("arity", &self.arity)
            .field("db_name", &self.db_name)
            .field("default_value", &self.default_value)
            .field("model", &"#ModelWeakRef#")
            .field("is_unique", &self.is_unique)
            .field("read_only", &self.read_only)
            .finish()
    }
}

impl Eq for ScalarField {}

impl Hash for ScalarField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.type_identifier.hash(state);
        self.is_required.hash(state);
        self.is_list.hash(state);
        self.is_id.hash(state);
        self.is_auto_generated_int_id.hash(state);
        self.internal_enum.hash(state);
        self.behaviour.hash(state);
        self.is_unique.hash(state);
        self.model().hash(state);
        self.arity.hash(state);
        self.db_name.hash(state);
    }
}

impl PartialEq for ScalarField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.type_identifier == other.type_identifier
            && self.is_required == other.is_required
            && self.is_list == other.is_list
            && self.is_id == other.is_id
            && self.is_auto_generated_int_id == other.is_auto_generated_int_id
            && self.internal_enum == other.internal_enum
            && self.behaviour == other.behaviour
            && self.default_value == other.default_value
            && self.is_unique == other.is_unique
            && self.model() == other.model()
            && self.arity == other.arity
            && self.db_name == other.db_name
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldBehaviour {
    CreatedAt,
    UpdatedAt,
    ScalarList { strategy: ScalarListStrategy },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ScalarListStrategy {
    Embedded,
    Relation,
}

impl ScalarFieldTemplate {
    pub fn build(self, model: ModelWeakRef) -> ScalarFieldRef {
        let scalar = ScalarField {
            name: self.name,
            type_identifier: self.type_identifier,
            is_id: self.is_id,
            is_required: self.is_required,
            is_list: self.is_list,
            is_autoincrement: self.is_autoincrement,
            is_auto_generated_int_id: self.is_auto_generated_int_id,
            read_only: OnceCell::new(),
            is_unique: self.is_unique,
            internal_enum: self.internal_enum,
            behaviour: self.behaviour,
            arity: self.arity,
            db_name: self.db_name,
            default_value: self.default_value,
            model,
        };

        Arc::new(scalar)
    }
}

impl ScalarField {
    pub fn model(&self) -> ModelRef {
        self.model
            .upgrade()
            .expect("Model does not exist anymore. Parent model got deleted without deleting the child.")
    }

    pub fn internal_data_model(&self) -> InternalDataModelRef {
        self.model().internal_data_model()
    }

    /// A field is an ID field if the name is `id` or `_id` in legacy internal_data_models,
    /// or if the field has Id behaviour defined.
    pub fn is_id(&self) -> bool {
        if self.model().is_legacy() {
            self.name == ID_FIELD || self.name == EMBEDDED_ID_FIELD
        } else {
            self.is_id
        }
    }

    pub fn is_created_at(&self) -> bool {
        if self.model().is_legacy() {
            self.name == CREATED_AT_FIELD
        } else {
            match self.behaviour {
                Some(FieldBehaviour::CreatedAt) => true,
                _ => false,
            }
        }
    }

    pub fn is_updated_at(&self) -> bool {
        if self.model().is_legacy() {
            self.name == UPDATED_AT_FIELD
        } else {
            match self.behaviour {
                Some(FieldBehaviour::UpdatedAt) => true,
                _ => false,
            }
        }
    }

    pub fn unique(&self) -> bool {
        self.is_unique || self.is_id()
    }

    pub fn db_name(&self) -> &str {
        &self.db_name.as_ref().unwrap_or(&self.name)
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        (self.type_identifier.clone(), self.arity)
    }

    pub fn is_read_only(&self) -> bool {
        self.read_only.get_or_init(|| false).clone()
    }
}

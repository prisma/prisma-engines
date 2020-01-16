use crate::prelude::*;
use datamodel::{DataSourceField, DefaultValue, FieldArity};
use std::{
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
    pub is_hidden: bool,
    pub is_auto_generated_int_id: bool,
    pub behaviour: Option<FieldBehaviour>,
    pub internal_enum: Option<InternalEnum>,
    pub data_source_field: DataSourceField,
}

#[derive(DebugStub)]
pub struct ScalarField {
    pub name: String,
    pub type_identifier: TypeIdentifier,
    pub is_required: bool,
    pub is_list: bool,
    pub is_hidden: bool,
    pub is_auto_generated_int_id: bool,
    pub internal_enum: Option<InternalEnum>,
    pub behaviour: Option<FieldBehaviour>,
    pub data_source_field: DataSourceField,

    #[debug_stub = "#ModelWeakRef#"]
    pub model: ModelWeakRef,

    pub(crate) is_unique: bool,
}

impl Eq for ScalarField {}

impl Hash for ScalarField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.type_identifier.hash(state);
        self.is_required.hash(state);
        self.is_list.hash(state);
        self.is_hidden.hash(state);
        self.is_auto_generated_int_id.hash(state);
        self.internal_enum.hash(state);
        self.behaviour.hash(state);
        self.is_unique.hash(state);
        self.model().hash(state);
    }
}

impl PartialEq for ScalarField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.type_identifier == other.type_identifier
            && self.is_required == other.is_required
            && self.is_list == other.is_list
            && self.is_hidden == other.is_hidden
            && self.is_auto_generated_int_id == other.is_auto_generated_int_id
            && self.data_source_field == other.data_source_field
            && self.internal_enum == other.internal_enum
            && self.behaviour == other.behaviour
            && self.default_value() == other.default_value()
            && self.is_unique == other.is_unique
            && self.model() == other.model()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FieldBehaviour {
    CreatedAt,
    UpdatedAt,
    Id {
        strategy: IdStrategy,
        sequence: Option<Sequence>, // TODO: this can be removed when we have switched fully to datamodel v2. This is not of interested for the query engine.
    },
    ScalarList {
        strategy: ScalarListStrategy,
    },
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum IdStrategy {
    Auto,
    None,
    Sequence,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum ScalarListStrategy {
    Embedded,
    Relation,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Sequence {
    pub name: String,
    pub initial_value: i32,
    pub allocation_size: i32,
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
            match self.behaviour {
                Some(FieldBehaviour::Id { .. }) => true,
                _ => false,
            }
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
        &self.data_source_field.name
    }

    pub fn id_behaviour_clone(&self) -> Option<FieldBehaviour> {
        if self.is_id() {
            self.behaviour.clone()
        } else {
            None
        }
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        let arity = match (self.is_list, self.is_required) {
            (true, _) => FieldArity::List,
            (false, true) => FieldArity::Required,
            (false, false) => FieldArity::Optional,
        };

        (self.type_identifier, arity)
    }

    pub fn default_value(&self) -> Option<&DefaultValue> {
        self.data_source_field.default_value.as_ref()
    }
}

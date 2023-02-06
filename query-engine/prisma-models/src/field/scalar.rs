use crate::{ast, parent_container::ParentContainer, prelude::*};
use dml::{DefaultValue, FieldArity, NativeTypeInstance};
use once_cell::sync::OnceCell;
use std::{
    fmt::{Debug, Display},
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

pub type ScalarFieldRef = Arc<ScalarField>;
pub type ScalarFieldWeak = Weak<ScalarField>;

pub struct ScalarField {
    pub(crate) name: String,
    pub(crate) type_identifier: TypeIdentifier,
    pub(crate) is_id: bool,
    pub(crate) is_auto_generated_int_id: bool,
    pub(crate) is_autoincrement: bool,
    pub(crate) is_updated_at: bool,
    pub(crate) internal_enum: Option<ast::EnumId>,
    pub(crate) arity: FieldArity,
    pub(crate) db_name: Option<String>,
    pub(crate) default_value: Option<DefaultValue>,
    pub(crate) native_type: Option<NativeTypeInstance>,
    pub(crate) container: ParentContainer,
    pub(crate) is_unique: bool,
    pub(crate) read_only: OnceCell<bool>,
}

impl ScalarField {
    pub fn internal_data_model(&self) -> InternalDataModelRef {
        self.container.internal_data_model()
    }

    pub fn is_id(&self) -> bool {
        self.is_id
    }

    pub fn is_list(&self) -> bool {
        matches!(self.arity, FieldArity::List)
    }

    pub fn is_required(&self) -> bool {
        matches!(self.arity, FieldArity::Required)
    }

    pub fn unique(&self) -> bool {
        self.is_unique || self.is_id()
    }

    pub fn db_name(&self) -> &str {
        self.db_name.as_deref().unwrap_or(self.name.as_str())
    }

    pub fn type_identifier_with_arity(&self) -> (TypeIdentifier, FieldArity) {
        (self.type_identifier.clone(), self.arity)
    }

    pub fn is_read_only(&self) -> bool {
        *self.read_only.get_or_init(|| false)
    }

    pub fn is_numeric(&self) -> bool {
        self.type_identifier.is_numeric()
    }

    pub fn container(&self) -> &ParentContainer {
        &self.container
    }

    pub fn name(&self) -> &str {
        self.name.as_ref()
    }

    pub fn type_identifier(&self) -> TypeIdentifier {
        self.type_identifier.clone()
    }

    pub fn arity(&self) -> FieldArity {
        self.arity
    }

    pub fn internal_enum(&self) -> Option<crate::InternalEnum> {
        self.internal_enum.map(|id| self.internal_data_model().zip(id))
    }

    pub fn default_value(&self) -> Option<&DefaultValue> {
        self.default_value.as_ref()
    }

    pub fn is_updated_at(&self) -> bool {
        self.is_updated_at
    }

    pub fn is_auto_generated_int_id(&self) -> bool {
        self.is_auto_generated_int_id
    }

    pub fn native_type(&self) -> Option<&NativeTypeInstance> {
        self.native_type.as_ref()
    }

    pub fn is_autoincrement(&self) -> bool {
        self.is_autoincrement
    }
}

impl Debug for ScalarField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScalarField")
            .field("name", &self.name)
            .field("type_identifier", &self.type_identifier)
            .field("is_id", &self.is_id)
            .field("is_auto_generated_int_id", &self.is_auto_generated_int_id)
            .field("is_autoincrement", &self.is_autoincrement)
            .field("internal_enum", &self.internal_enum)
            .field("is_updated_at", &self.is_updated_at)
            .field("arity", &self.arity)
            .field("db_name", &self.db_name)
            .field("default_value", &self.default_value)
            .field("container", &self.container().name())
            .field("is_unique", &self.is_unique)
            .field("read_only", &self.read_only)
            .finish()
    }
}

impl Display for ScalarField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}", self.container().name(), self.name)
    }
}

impl Eq for ScalarField {}

impl Hash for ScalarField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.type_identifier.hash(state);
        self.is_id.hash(state);
        self.is_auto_generated_int_id.hash(state);
        self.internal_enum.hash(state);
        self.is_updated_at.hash(state);
        self.is_unique.hash(state);
        self.container.hash(state);
        self.arity.hash(state);
        self.db_name.hash(state);
    }
}

impl PartialEq for ScalarField {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.type_identifier == other.type_identifier
            && self.is_id == other.is_id
            && self.is_auto_generated_int_id == other.is_auto_generated_int_id
            && self.internal_enum == other.internal_enum
            && self.is_updated_at == other.is_updated_at
            && self.default_value == other.default_value
            && self.is_unique == other.is_unique
            && self.container == other.container
            && self.arity == other.arity
            && self.db_name == other.db_name
    }
}

use super::*;
use crate::native_type_instance::NativeTypeInstance;
use crate::scalars::ScalarType;
use crate::traits::{Ignorable, WithDatabaseName, WithName};
use crate::{
    default_value::{DefaultValue, ValueGenerator},
    relation_info::ReferentialAction,
};
use std::hash::Hash;

/// Arity of a Field in a Model.
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

impl FieldArity {
    pub fn is_list(&self) -> bool {
        self == &Self::List
    }

    pub fn is_required(&self) -> bool {
        self == &Self::Required
    }

    pub fn is_optional(&self) -> bool {
        self == &Self::Optional
    }
}

/// Datamodel field type.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    /// This is an enum field, with an enum of the given name.
    Enum(String),
    /// This is a relation field.
    Relation(RelationInfo),
    /// This is a field with an unsupported datatype. The content is the db's description of the type, it should enable migrate to create the type.
    Unsupported(String),
    /// The first option is Some(x) if the scalar type is based upon a type alias.
    Scalar(ScalarType, Option<String>, Option<NativeTypeInstance>),
}

impl FieldType {
    pub fn as_scalar(&self) -> Option<&ScalarType> {
        match self {
            FieldType::Scalar(scalar_type, _, _) => Some(scalar_type),
            _ => None,
        }
    }

    pub fn as_native_type(&self) -> Option<(&ScalarType, &NativeTypeInstance)> {
        match self {
            FieldType::Scalar(a, _, Some(b)) => Some((a, b)),
            _ => None,
        }
    }

    pub fn is_compatible_with(&self, other: &FieldType) -> bool {
        match (self, other) {
            (Self::Scalar(a, _, nta), Self::Scalar(b, _, ntb)) => a == b && nta == ntb, // the name of the type alias is not important for the comparison
            (a, b) => a == b,
        }
    }

    pub fn is_datetime(&self) -> bool {
        self.scalar_type().map(|st| st.is_datetime()).unwrap_or(false)
    }

    pub fn is_string(&self) -> bool {
        self.scalar_type().map(|st| st.is_string()).unwrap_or(false)
    }

    pub fn is_enum(&self, name: &str) -> bool {
        matches!(self, Self::Enum(this) if this == name)
    }

    pub fn scalar_type(&self) -> Option<ScalarType> {
        match self {
            FieldType::Scalar(st, _, _) => Some(*st),
            _ => None,
        }
    }

    pub fn native_type(&self) -> Option<&NativeTypeInstance> {
        match self {
            FieldType::Scalar(_, _, Some(nt)) => Some(nt),
            _ => None,
        }
    }
}

/// Represents a Field in a Model.
#[derive(Debug, PartialEq, Clone)]
pub enum Field {
    ScalarField(ScalarField),
    RelationField(RelationField),
}

impl Field {
    pub fn as_relation_field(&self) -> Option<&RelationField> {
        match self {
            Field::RelationField(sf) => Some(sf),
            _ => None,
        }
    }

    pub fn as_relation_field_mut(&mut self) -> Option<&mut RelationField> {
        match self {
            Field::RelationField(ref mut rf) => Some(rf),
            _ => None,
        }
    }

    pub fn as_scalar_field(&self) -> Option<&ScalarField> {
        match self {
            Field::ScalarField(sf) => Some(sf),
            _ => None,
        }
    }

    pub fn is_relation(&self) -> bool {
        matches!(self, Field::RelationField(_))
    }

    pub fn is_scalar_field(&self) -> bool {
        matches!(self, Field::ScalarField(_))
    }

    pub fn name(&self) -> &str {
        match &self {
            Field::ScalarField(sf) => &sf.name,
            Field::RelationField(rf) => &rf.name,
        }
    }

    pub fn documentation(&self) -> Option<&str> {
        match self {
            Field::ScalarField(sf) => sf.documentation.as_deref(),
            Field::RelationField(rf) => rf.documentation.as_deref(),
        }
    }

    pub fn set_documentation(&mut self, documentation: Option<String>) {
        match self {
            Field::ScalarField(sf) => sf.documentation = documentation,
            Field::RelationField(rf) => rf.documentation = documentation,
        }
    }

    pub fn is_commented_out(&self) -> bool {
        match self {
            Field::ScalarField(sf) => sf.is_commented_out,
            Field::RelationField(rf) => rf.is_commented_out,
        }
    }

    pub fn arity(&self) -> &FieldArity {
        match &self {
            Field::ScalarField(sf) => &sf.arity,
            Field::RelationField(rf) => &rf.arity,
        }
    }

    pub fn field_type(&self) -> FieldType {
        match &self {
            Field::ScalarField(sf) => sf.field_type.clone(),
            Field::RelationField(rf) => FieldType::Relation(rf.relation_info.clone()),
        }
    }

    pub fn default_value(&self) -> Option<&DefaultValue> {
        match &self {
            Field::ScalarField(sf) => sf.default_value.as_ref(),
            Field::RelationField(_) => None,
        }
    }

    pub fn is_updated_at(&self) -> bool {
        match &self {
            Field::ScalarField(sf) => sf.is_updated_at,
            Field::RelationField(_) => false,
        }
    }

    pub fn is_unique(&self) -> bool {
        match &self {
            Field::ScalarField(sf) => sf.is_unique,
            Field::RelationField(_) => false,
        }
    }

    pub fn is_id(&self) -> bool {
        match &self {
            Field::ScalarField(sf) => sf.is_id,
            Field::RelationField(_) => false,
        }
    }

    pub fn is_generated(&self) -> bool {
        match &self {
            Field::ScalarField(sf) => sf.is_generated,
            Field::RelationField(rf) => rf.is_generated,
        }
    }
}

impl WithName for Field {
    fn name(&self) -> &String {
        match self {
            Field::ScalarField(sf) => sf.name(),
            Field::RelationField(rf) => rf.name(),
        }
    }
    fn set_name(&mut self, name: &str) {
        match self {
            Field::ScalarField(sf) => sf.set_name(name),
            Field::RelationField(rf) => rf.set_name(name),
        }
    }
}

impl WithDatabaseName for Field {
    fn database_name(&self) -> Option<&str> {
        match self {
            Field::ScalarField(sf) => sf.database_name.as_deref(),
            Field::RelationField(_) => None,
        }
    }

    fn set_database_name(&mut self, database_name: Option<String>) {
        match self {
            Field::ScalarField(sf) => sf.set_database_name(database_name),
            Field::RelationField(_) => (),
        }
    }
}

/// Represents a relation field in a model.
#[derive(Debug, Clone)]
pub struct RelationField {
    /// Name of the field.
    pub name: String,

    /// The field's type.
    pub relation_info: RelationInfo,

    /// The field's arity.
    pub arity: FieldArity,

    /// The arity of underlying fields for referential actions.
    pub referential_arity: FieldArity,

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// signals that this field was internally generated (only back relation fields as of now)
    pub is_generated: bool,

    /// Indicates if this field has to be commented out.
    pub is_commented_out: bool,

    /// Indicates if this field has to be ignored by the Client.
    pub is_ignored: bool,

    /// Is `ON DELETE/UPDATE RESTRICT` allowed.
    pub supports_restrict_action: Option<bool>,

    /// Do we run the referential actions in the core instead of the database.
    pub emulates_referential_actions: Option<bool>,
}

impl PartialEq for RelationField {
    //ignores the relation name for reintrospection
    fn eq(&self, other: &Self) -> bool {
        let this_matches = self.name == other.name
            && self.arity == other.arity
            && self.referential_arity == other.referential_arity
            && self.documentation == other.documentation
            && self.is_generated == other.is_generated
            && self.is_commented_out == other.is_commented_out
            && self.is_ignored == other.is_ignored
            && self.relation_info == other.relation_info;

        let this_on_delete = self
            .relation_info
            .on_delete
            .unwrap_or_else(|| self.default_on_delete_action());

        let other_on_delete = other
            .relation_info
            .on_delete
            .unwrap_or_else(|| other.default_on_delete_action());

        let on_delete_matches = this_on_delete == other_on_delete;

        let this_on_update = self
            .relation_info
            .on_update
            .unwrap_or_else(|| self.default_on_update_action());

        let other_on_update = other
            .relation_info
            .on_update
            .unwrap_or_else(|| other.default_on_update_action());

        let on_update_matches = this_on_update == other_on_update;

        this_matches && on_delete_matches && on_update_matches
    }
}

impl RelationField {
    /// Creates a new field with the given name and type.
    pub fn new(name: &str, arity: FieldArity, referential_arity: FieldArity, relation_info: RelationInfo) -> Self {
        RelationField {
            name: String::from(name),
            arity,
            referential_arity,
            relation_info,
            documentation: None,
            is_generated: false,
            is_commented_out: false,
            is_ignored: false,
            supports_restrict_action: None,
            emulates_referential_actions: None,
        }
    }

    /// The default `onDelete` can be `Restrict`.
    pub fn supports_restrict_action(&mut self, value: bool) {
        self.supports_restrict_action = Some(value);
    }

    /// The referential actions should be handled by the core.
    pub fn emulates_referential_actions(&mut self, value: bool) {
        self.emulates_referential_actions = Some(value);
    }

    /// Creates a new field with the given name and type, marked as generated and optional.
    pub fn new_generated(name: &str, info: RelationInfo, required: bool) -> Self {
        let arity = if required {
            FieldArity::Required
        } else {
            FieldArity::Optional
        };

        let mut field = Self::new(name, arity, arity, info);
        field.is_generated = true;

        field
    }

    pub fn points_to_model(&self, name: &str) -> bool {
        self.relation_info.to == name
    }

    pub fn is_required(&self) -> bool {
        self.arity.is_required()
    }

    pub fn is_list(&self) -> bool {
        self.arity.is_list()
    }

    pub fn is_singular(&self) -> bool {
        !self.is_list()
    }

    pub fn is_optional(&self) -> bool {
        self.arity.is_optional()
    }

    pub fn default_on_delete_action(&self) -> ReferentialAction {
        use ReferentialAction::*;

        match self.referential_arity {
            FieldArity::Required if self.supports_restrict_action.unwrap_or(true) => Restrict,
            FieldArity::Required => NoAction,
            _ => SetNull,
        }
    }

    pub fn default_on_update_action(&self) -> ReferentialAction {
        use ReferentialAction::*;

        match self.referential_arity {
            _ if !self.emulates_referential_actions.unwrap_or(false) => Cascade,
            FieldArity::Required => Restrict,
            _ => SetNull,
        }
    }
}

/// Represents a scalar field in a model.
#[derive(Debug, PartialEq, Clone)]
pub struct ScalarField {
    /// Name of the field.
    pub name: String,

    /// The field's type.
    pub field_type: FieldType,

    /// The field's arity.
    pub arity: FieldArity,

    /// The database internal name.
    pub database_name: Option<String>,

    /// The default value.
    pub default_value: Option<DefaultValue>,

    /// Indicates if the field is unique.
    pub is_unique: bool,

    /// true if this field marked with @id.
    pub is_id: bool,

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// signals that this field was internally generated (only back relation fields as of now)
    pub is_generated: bool,

    /// If set, signals that this field is updated_at and will be updated to now()
    /// automatically.
    pub is_updated_at: bool,

    /// Indicates if this field has to be commented out.
    pub is_commented_out: bool,

    /// Indicates if this field is ignored by the Client.
    pub is_ignored: bool,
}

impl ScalarField {
    /// Creates a new field with the given name and type.
    pub fn new(name: &str, arity: FieldArity, field_type: FieldType) -> ScalarField {
        ScalarField {
            name: String::from(name),
            arity,
            field_type,
            database_name: None,
            default_value: None,
            is_unique: false,
            is_id: false,
            documentation: None,
            is_generated: false,
            is_updated_at: false,
            is_commented_out: false,
            is_ignored: false,
        }
    }
    /// Creates a new field with the given name and type, marked as generated and optional.
    pub fn new_generated(name: &str, field_type: FieldType) -> ScalarField {
        let mut field = Self::new(name, FieldArity::Optional, field_type);
        field.is_generated = true;

        field
    }

    //todo use withdatabasename::final_database_name instead
    pub fn db_name(&self) -> &str {
        self.database_name.as_ref().unwrap_or(&self.name)
    }

    pub fn is_required(&self) -> bool {
        self.arity.is_required()
    }

    pub fn is_list(&self) -> bool {
        self.arity.is_list()
    }

    pub fn is_singular(&self) -> bool {
        !self.is_list()
    }

    pub fn is_optional(&self) -> bool {
        self.arity.is_optional()
    }

    pub fn is_auto_increment(&self) -> bool {
        matches!(&self.default_value, Some(DefaultValue::Expression(expr)) if expr == &ValueGenerator::new_autoincrement())
    }
}

impl WithName for ScalarField {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithName for RelationField {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for ScalarField {
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }
    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}

impl Ignorable for Field {
    fn is_ignored(&self) -> bool {
        match self {
            Field::RelationField(rf) => rf.is_ignored,
            Field::ScalarField(sf) => sf.is_ignored,
        }
    }

    fn ignore(&mut self) {
        match self {
            Field::RelationField(rf) => rf.is_ignored = true,
            Field::ScalarField(sf) => sf.is_ignored = true,
        }
    }
}

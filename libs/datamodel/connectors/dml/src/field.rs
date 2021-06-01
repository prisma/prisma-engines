use super::*;
use crate::default_value::{DefaultValue, ValueGenerator};
use crate::model::{IndexDefinition, PrimaryKeyDefinition};
use crate::native_type_instance::NativeTypeInstance;
use crate::scalars::ScalarType;
use crate::traits::{Ignorable, WithDatabaseName, WithName};
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

// TODO: when progressing with the native types implementation we should consider merging the variants `NativeType` and `Base`
//Agreed
/// Datamodel field type.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    /// This is an enum field, with an enum of the given name.
    Enum(String),
    /// This is a relation field.
    Relation(RelationInfo),
    /// native field type.
    NativeType(ScalarType, NativeTypeInstance),
    /// This is a field with an unsupported datatype. The content is the db's description of the type, it should enable migrate to create the type.
    Unsupported(String),
    /// The option is Some(x) if the scalar type is based upon a type alias.
    Base(ScalarType, Option<String>),
}

impl FieldType {
    pub fn as_base(&self) -> Option<&ScalarType> {
        match self {
            FieldType::Base(scalar_type, _) => Some(scalar_type),
            _ => None,
        }
    }

    pub fn as_native_type(&self) -> Option<(&ScalarType, &NativeTypeInstance)> {
        match self {
            FieldType::NativeType(a, b) => Some((a, b)),
            _ => None,
        }
    }

    pub fn is_compatible_with(&self, other: &FieldType) -> bool {
        match (self, other) {
            (Self::Base(a, _), Self::Base(b, _)) => a == b, // the name of the type alias is not important for the comparison
            (a, b) => a == b,
        }
    }

    pub fn is_datetime(&self) -> bool {
        self.scalar_type().map(|st| st.is_datetime()).unwrap_or(false)
    }

    pub fn is_string(&self) -> bool {
        self.scalar_type().map(|st| st.is_string()).unwrap_or(false)
    }

    pub fn scalar_type(&self) -> Option<ScalarType> {
        match self {
            FieldType::NativeType(st, _) => Some(*st),
            FieldType::Base(st, _) => Some(*st),
            _ => None,
        }
    }

    pub fn native_type(&self) -> Option<&NativeTypeInstance> {
        match self {
            FieldType::NativeType(_, nt) => Some(nt),
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
            Field::ScalarField(sf) => sf.is_unique.is_some(),
            Field::RelationField(_) => false,
        }
    }

    pub fn is_id(&self) -> bool {
        match &self {
            Field::ScalarField(sf) => sf.primary_key.is_some(),
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
#[derive(Debug, PartialEq, Clone)]
pub struct RelationField {
    /// Name of the field.
    pub name: String,

    /// The field's type.
    pub relation_info: RelationInfo,

    /// The field's arity.
    pub arity: FieldArity,

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// signals that this field was internally generated (only back relation fields as of now)
    pub is_generated: bool,

    /// Indicates if this field has to be commented out.
    pub is_commented_out: bool,

    /// Indicates if this field has to be ignored by the Client.
    pub is_ignored: bool,
}

impl RelationField {
    /// Creates a new field with the given name and type.
    pub fn new(name: &str, arity: FieldArity, relation_info: RelationInfo) -> Self {
        RelationField {
            name: String::from(name),
            arity,
            relation_info,
            documentation: None,
            is_generated: false,
            is_commented_out: false,
            is_ignored: false,
        }
    }
    /// Creates a new field with the given name and type, marked as generated and optional.
    pub fn new_generated(name: &str, info: RelationInfo, required: bool) -> Self {
        let arity = if required {
            FieldArity::Required
        } else {
            FieldArity::Optional
        };

        let mut field = Self::new(name, arity, info);
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

    //todo idealy we get rid of this on the field level and only have it on the model level or only have it as a bool
    /// Indicates if the field is unique.
    pub is_unique: Option<IndexDefinition>,

    //todo idealy we get rid of this on the field level and only have it on the model level  or only have it as a bool
    /// The Primary Key definition if the PK covers only this field .
    pub primary_key: Option<PrimaryKeyDefinition>,

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
            is_unique: None,
            primary_key: None,
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

    pub fn is_id(&self) -> bool {
        self.primary_key.is_some()
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

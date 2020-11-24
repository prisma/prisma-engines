use super::*;
use crate::default_value::{DefaultValue, ValueGenerator};
use crate::native_type_instance::NativeTypeInstance;
use crate::scalars::ScalarType;
use crate::traits::{WithDatabaseName, WithName};
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
/// Datamodel field type.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    /// This is an enum field, with an enum of the given name.
    Enum(String),
    /// This is a relation field.
    Relation(RelationInfo),
    /// native field type.
    NativeType(ScalarType, NativeTypeInstance),
    /// This is a field with an unsupported datatype - used by introspection only.
    Unsupported(String),
    /// The option is Some(x) if the scalar type is based upon a type alias.
    Base(ScalarType, Option<String>),
}

impl FieldType {
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Base(a, _), Self::Base(b, _)) => a == b, // the name of the type alias is not important for the comparison
            (a, b) => a == b,
        }
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
    pub fn is_relation(&self) -> bool {
        match self {
            Field::ScalarField(_) => false,
            Field::RelationField(_) => true,
        }
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
        }
    }
    /// Creates a new field with the given name and type, marked as generated and optional.
    pub fn new_generated(name: &str, info: RelationInfo) -> Self {
        let mut field = Self::new(name, FieldArity::Optional, info);
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

    /// A relation field is virtual if there's no reference to the related model stored in this model.
    /// example: In SQL this means that this will return true if the foreign key is stored on the other side.
    pub fn is_virtual(&self) -> bool {
        self.relation_info.fields.is_empty() && self.relation_info.to_fields.is_empty()
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

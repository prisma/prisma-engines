use super::*;
use datamodel_connector::ScalarFieldType;
use std::hash::Hash;

/// Datamodel field arity.
#[derive(Debug, PartialEq, Copy, Clone, Eq, Hash)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

impl FieldArity {
    pub fn is_singular(&self) -> bool {
        self == &FieldArity::Required || self == &FieldArity::Optional
    }

    pub fn verbal_display(&self) -> &'static str {
        match self {
            FieldArity::Required => "required",
            FieldArity::Optional => "optional",
            FieldArity::List => "list",
        }
    }

    pub fn is_required(&self) -> bool {
        self == &Self::Required
    }

    pub fn is_optional(&self) -> bool {
        self == &Self::Optional
    }

    pub fn is_list(&self) -> bool {
        self == &Self::List
    }
}

/// Datamodel field type.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    /// This is an enum field, with an enum of the given name.
    Enum(String),
    /// This is a relation field.
    Relation(RelationInfo),
    /// Connector specific field type.
    ConnectorSpecific(ScalarFieldType),
    /// This is a field with an unsupported datatype.
    Unsupported(String),
    /// The option is Some(x) if the scalar type is based upon a type alias.
    Base(ScalarType, Option<String>),
}

impl FieldType {
    pub fn is_relation(&self) -> bool {
        match self {
            Self::Relation(_) => true,
            _ => false,
        }
    }

    pub fn is_compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Base(a, _), Self::Base(b, _)) => a == b, // the name of the type alias is not important for the comparison
            (a, b) => a == b,
        }
    }

    pub fn scalar_type(&self) -> Option<ScalarType> {
        match self {
            FieldType::ConnectorSpecific(sft) => Some(sft.prisma_type()),
            FieldType::Base(st, _) => Some(*st),
            _ => None,
        }
    }
}

/// Represents a field in a model.
#[derive(Debug, PartialEq, Clone)]
pub struct Field {
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

impl Field {
    pub fn points_to_model(&self, name: &str) -> bool {
        match &self.field_type {
            FieldType::Relation(rel_info) if rel_info.to == name => true,
            _ => false,
        }
    }

    pub fn db_name(&self) -> &str {
        self.database_name.as_ref().unwrap_or(&self.name)
    }

    pub fn is_relation(&self) -> bool {
        match self.field_type {
            FieldType::Relation(_) => true,
            _ => false,
        }
    }
}

impl WithName for Field {
    fn name(&self) -> &String {
        &self.name
    }
    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for Field {
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }

    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}

impl Field {
    /// Creates a new field with the given name and type.
    pub fn new(name: &str, field_type: FieldType) -> Field {
        Field {
            name: String::from(name),
            arity: FieldArity::Required,
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
    pub fn new_generated(name: &str, field_type: FieldType) -> Field {
        let mut field = Self::new(name, field_type);
        field.arity = FieldArity::Optional;
        field.is_generated = true;

        field
    }
}

use super::*;
use datamodel_connector::ScalarFieldType;

/// Datamodel field arity.
#[derive(Debug, PartialEq, Copy, Clone)]
pub enum FieldArity {
    Required,
    Optional,
    List,
}

impl FieldArity {
    pub fn is_singular(&self) -> bool {
        self == &FieldArity::Required || self == &FieldArity::Optional
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
    /// Base (built-in scalar) type.
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
}

/// Represents a field in a model.
#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    /// Name of the field.
    pub name: String,
    /// The field's arity.
    pub arity: FieldArity,
    /// The field's type.
    pub field_type: FieldType,
    /// The database internal name.
    pub database_names: Vec<String>,
    /// The default value.
    pub default_value: Option<DefaultValue>,
    /// Indicates if the field is unique.
    pub is_unique: bool,
    /// If set, signals that this field is an id field, or
    /// primary key.
    pub is_id: bool,
    /// Comments associated with this field.
    pub documentation: Option<String>,
    /// If set, signals that this field was internally generated
    /// and should never be displayed to the user.
    pub is_generated: bool,
    /// If set, signals that this field is updated_at and will be updated to now()
    /// automatically.
    pub is_updated_at: bool,
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
    fn database_names(&self) -> Vec<&str> {
        self.database_names.iter().map(|s| s.as_str()).collect()
    }

    fn set_database_names(&mut self, database_names: Vec<String>) -> Result<(), String> {
        match &self.field_type {
            FieldType::Relation(rel_info) => {
                let num_of_to_fields = rel_info.to_fields.len();
                // in case of auto populated to fields the validation is very hard. We want to move to explicit references anyway.
                // TODO: revisist this once explicit `@relation(references:)` is implemented
                let should_validate = num_of_to_fields > 0;
                if should_validate && rel_info.to_fields.len() != database_names.len() {
                    Err(format!(
                        "This Relation Field must specify exactly {} mapped names.",
                        rel_info.to_fields.len()
                    ))
                } else {
                    self.database_names = database_names;
                    Ok(())
                }
            }
            _ => {
                if database_names.len() > 1 {
                    Err("A scalar Field must not specify multiple mapped names.".to_string())
                } else {
                    self.database_names = database_names;
                    Ok(())
                }
            }
        }
    }
}

impl Field {
    /// Creates a new field with the given name and type.
    pub fn new(name: &str, field_type: FieldType) -> Field {
        Field {
            name: String::from(name),
            arity: FieldArity::Required,
            field_type,
            database_names: Vec::new(),
            default_value: None,
            is_unique: false,
            is_id: false,
            documentation: None,
            is_generated: false,
            is_updated_at: false,
        }
    }
    /// Creates a new field with the given name and type, marked as generated and optional.
    pub fn new_generated(name: &str, field_type: FieldType) -> Field {
        Field {
            name: String::from(name),
            arity: FieldArity::Optional,
            field_type,
            database_names: Vec::new(),
            default_value: None,
            is_unique: false,
            is_id: false,
            documentation: None,
            is_generated: true,
            is_updated_at: false,
        }
    }
}

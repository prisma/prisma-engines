use super::*;
use datamodel_connector::ScalarFieldType;
use std::hash::{Hash, Hasher};

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
}

/// Describes a singular field on a data source.
/// This doesn't necessarily map 1:1 to fields in the datamodel, as some
/// datamodel fields, notably relation fields, can be backed by multiple
/// data source fields.
#[derive(Debug, PartialEq, Clone)]
pub struct DataSourceField {
    /// Name of the backing data source field (e.g. SQL column name or document key).
    pub name: String,
    pub field_type: ScalarType,
    pub arity: FieldArity,
    pub default_value: Option<DefaultValue>,
}

impl Hash for DataSourceField {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.name.hash(state);
        self.field_type.hash(state);
        self.arity.hash(state);
    }
}

impl Eq for DataSourceField {}

/// Represents a field in a model.
#[derive(Debug, PartialEq, Clone)]
pub struct Field {
    /// Name of the field.
    pub name: String,

    /// The field's type.
    pub field_type: FieldType,

    // -------- todo this is duplicated from DataSourceField --------
    /// The field's arity.
    pub arity: FieldArity,

    /// The database internal name.
    pub database_names: Vec<String>,

    /// The default value.
    pub default_value: Option<DefaultValue>,
    // -------- -------------------------------------------- --------
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

    /// The data source field specifics, like backing fields and defaults.
    pub data_source_fields: Vec<DataSourceField>,

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
            data_source_fields: vec![],
            is_commented_out: false,
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
            data_source_fields: vec![],
            is_commented_out: false,
        }
    }
}

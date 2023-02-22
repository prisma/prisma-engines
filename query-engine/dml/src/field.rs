//! A field in a model.

use crate::default_value::{DefaultKind, DefaultValue, ValueGenerator};
use crate::native_type_instance::NativeTypeInstance;
use crate::relation_info::RelationInfo;
use crate::scalars::ScalarType;
use crate::traits::{WithDatabaseName, WithName};
use crate::FieldArity;
use psl_core::{parser_database::walkers::ScalarFieldId, schema_ast::ast};

/// Datamodel field type.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldType {
    /// This is an enum field, with an enum of the given name.
    Enum(ast::EnumId),
    /// This is a relation field.
    Relation(RelationInfo),
    /// This is a field with an unsupported datatype. The content is the db's description of the type, it should enable migrate to create the type.
    Unsupported(String),
    Scalar(ScalarType, Option<NativeTypeInstance>),
    /// This is a composite type fields, with a composite type of the given type.
    CompositeType(String),
}

impl FieldType {
    pub fn as_enum(&self) -> Option<ast::EnumId> {
        match self {
            FieldType::Enum(id) => Some(*id),
            _ => None,
        }
    }

    pub fn as_scalar(&self) -> Option<&ScalarType> {
        match self {
            FieldType::Scalar(scalar_type, _) => Some(scalar_type),
            _ => None,
        }
    }

    pub fn as_native_type(&self) -> Option<(&ScalarType, &NativeTypeInstance)> {
        match self {
            FieldType::Scalar(a, Some(b)) => Some((a, b)),
            _ => None,
        }
    }

    pub fn as_composite_type(&self) -> Option<&str> {
        match self {
            FieldType::CompositeType(ct) => Some(ct),
            _ => None,
        }
    }

    pub fn is_datetime(&self) -> bool {
        self.scalar_type().map(|st| st.is_datetime()).unwrap_or(false)
    }

    pub fn is_string(&self) -> bool {
        self.scalar_type().map(|st| st.is_string()).unwrap_or(false)
    }

    pub fn is_composite(&self) -> bool {
        matches!(self, Self::CompositeType(_))
    }

    pub fn scalar_type(&self) -> Option<ScalarType> {
        match self {
            FieldType::Scalar(st, _) => Some(*st),
            _ => None,
        }
    }

    pub fn native_type(&self) -> Option<&NativeTypeInstance> {
        match self {
            FieldType::Scalar(_, Some(nt)) => Some(nt),
            _ => None,
        }
    }
}

pub type Field = ScalarField;

/// Represents a scalar field in a model.
#[derive(Debug, PartialEq, Clone)]
pub struct ScalarField {
    pub id: ScalarFieldId,

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

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// signals that this field was internally generated (only back relation fields as of now)
    pub is_generated: bool,

    /// If set, signals that this field is updated_at and will be updated to now()
    /// automatically.
    pub is_updated_at: bool,

    /// Indicates if this field is ignored by the Client.
    pub is_ignored: bool,
}

impl ScalarField {
    /// Creates a new field with the given name and type.
    pub fn new(id: ScalarFieldId, name: &str, arity: FieldArity, field_type: FieldType) -> ScalarField {
        ScalarField {
            id,
            name: String::from(name),
            arity,
            field_type,
            database_name: None,
            default_value: None,
            documentation: None,
            is_generated: false,
            is_updated_at: false,
            is_ignored: false,
        }
    }

    pub fn set_default_value(&mut self, val: DefaultValue) {
        self.default_value = Some(val)
    }

    //todo use withdatabasename::final_database_name instead
    pub fn db_name(&self) -> &str {
        self.database_name.as_ref().unwrap_or(&self.name)
    }

    pub fn is_required(&self) -> bool {
        self.arity.is_required()
    }

    pub fn is_optional(&self) -> bool {
        self.arity.is_optional()
    }

    pub fn is_auto_increment(&self) -> bool {
        let kind = self.default_value().map(|val| &val.kind);
        matches!(kind, Some(DefaultKind::Expression(ref expr)) if expr == &ValueGenerator::new_autoincrement())
    }

    pub fn default_value(&self) -> Option<&DefaultValue> {
        self.default_value.as_ref()
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

impl WithDatabaseName for ScalarField {
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }
    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}

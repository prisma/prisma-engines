//! A field in a model.

use crate::default_value::{DefaultKind, DefaultValue, ValueGenerator};
use crate::native_type_instance::NativeTypeInstance;
use crate::relation_info::RelationInfo;
use crate::scalars::ScalarType;
use crate::traits::{Ignorable, WithDatabaseName, WithName};
use crate::{CompositeTypeFieldType, FieldArity};
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

impl From<CompositeTypeFieldType> for FieldType {
    fn from(typ: CompositeTypeFieldType) -> Self {
        match typ {
            CompositeTypeFieldType::CompositeType(t) => Self::CompositeType(t),
            CompositeTypeFieldType::Scalar(t, nt) => Self::Scalar(t, nt),
            CompositeTypeFieldType::Enum(e) => Self::Enum(e),
            CompositeTypeFieldType::Unsupported(u) => Self::Unsupported(u),
        }
    }
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

    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported(_))
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

/// Represents a Field in a Model.
#[derive(Debug, PartialEq, Clone)]
pub enum Field {
    ScalarField(ScalarField),
    CompositeField(CompositeField),
}

impl Field {
    pub fn as_scalar_field(&self) -> Option<&ScalarField> {
        match self {
            Field::ScalarField(sf) => Some(sf),
            _ => None,
        }
    }

    pub fn as_scalar_field_mut(&mut self) -> Option<&mut ScalarField> {
        match self {
            Field::ScalarField(sf) => Some(sf),
            _ => None,
        }
    }

    pub fn as_composite_field(&self) -> Option<&CompositeField> {
        match self {
            Field::CompositeField(cf) => Some(cf),
            _ => None,
        }
    }

    pub fn is_scalar_field(&self) -> bool {
        matches!(self, Field::ScalarField(_))
    }

    pub fn name(&self) -> &str {
        match &self {
            Field::ScalarField(sf) => &sf.name,
            Field::CompositeField(cf) => &cf.name,
        }
    }

    pub fn documentation(&self) -> Option<&str> {
        match self {
            Field::ScalarField(sf) => sf.documentation.as_deref(),
            Field::CompositeField(cf) => cf.documentation.as_deref(),
        }
    }

    pub fn set_documentation(&mut self, documentation: Option<String>) {
        match self {
            Field::ScalarField(sf) => sf.documentation = documentation,
            Field::CompositeField(cf) => cf.documentation = documentation,
        }
    }

    pub fn is_commented_out(&self) -> bool {
        match self {
            Field::ScalarField(sf) => sf.is_commented_out,
            Field::CompositeField(cf) => cf.is_commented_out,
        }
    }

    pub fn arity(&self) -> &FieldArity {
        match &self {
            Field::ScalarField(sf) => &sf.arity,
            Field::CompositeField(rf) => &rf.arity,
        }
    }

    pub fn field_type(&self) -> FieldType {
        match &self {
            Field::ScalarField(sf) => sf.field_type.clone(),
            Field::CompositeField(cf) => FieldType::CompositeType(cf.composite_type.clone()),
        }
    }

    pub fn default_value(&self) -> Option<&DefaultValue> {
        match &self {
            Field::ScalarField(sf) => sf.default_value.as_ref(),
            Field::CompositeField(_) => None,
        }
    }

    pub fn is_updated_at(&self) -> bool {
        match &self {
            Field::ScalarField(sf) => sf.is_updated_at,
            Field::CompositeField(_) => false,
        }
    }
}

impl WithName for Field {
    fn name(&self) -> &String {
        match self {
            Field::ScalarField(sf) => sf.name(),
            Field::CompositeField(cf) => cf.name(),
        }
    }
    fn set_name(&mut self, name: &str) {
        match self {
            Field::ScalarField(sf) => sf.set_name(name),
            Field::CompositeField(cf) => cf.set_name(name),
        }
    }
}

impl WithDatabaseName for Field {
    fn database_name(&self) -> Option<&str> {
        match self {
            Field::ScalarField(sf) => sf.database_name.as_deref(),
            Field::CompositeField(cf) => cf.database_name.as_deref(),
        }
    }

    fn set_database_name(&mut self, database_name: Option<String>) {
        match self {
            Field::ScalarField(sf) => sf.set_database_name(database_name),
            Field::CompositeField(cf) => cf.set_database_name(database_name),
        }
    }
}

impl Ignorable for Field {
    fn is_ignored(&self) -> bool {
        match self {
            Field::ScalarField(sf) => sf.is_ignored,
            Field::CompositeField(cf) => cf.is_ignored,
        }
    }

    fn ignore(&mut self) {
        match self {
            Field::ScalarField(sf) => sf.is_ignored = true,
            Field::CompositeField(cf) => cf.is_ignored = true,
        }
    }
}

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

    /// Indicates if this field has to be commented out.
    pub is_commented_out: bool,

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
            is_commented_out: false,
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
        let kind = self.default_value().map(|val| val.kind());
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

/// Represents a composite field.
#[derive(Debug, PartialEq, Clone)]
pub struct CompositeField {
    pub id: ScalarFieldId,

    /// Name of the field.
    pub name: String,

    /// The database internal name.
    pub database_name: Option<String>,

    /// The name of the composite type that backs this field.
    pub composite_type: String,

    /// The field's arity.
    pub arity: FieldArity,

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// Indicates if this field has to be commented out.
    pub is_commented_out: bool,

    /// Indicates if this field has to be ignored by the Client.
    pub is_ignored: bool,

    /// The default value of this field
    pub default_value: Option<DefaultValue>,
}

impl CompositeField {
    pub fn new(id: ScalarFieldId) -> Self {
        CompositeField {
            id,
            name: String::new(),
            database_name: None,
            composite_type: String::new(),
            arity: FieldArity::Optional,
            documentation: None,
            is_commented_out: false,
            is_ignored: false,
            default_value: None,
        }
    }
}

impl WithName for CompositeField {
    fn name(&self) -> &String {
        &self.name
    }

    fn set_name(&mut self, name: &str) {
        self.name = String::from(name)
    }
}

impl WithDatabaseName for CompositeField {
    fn database_name(&self) -> Option<&str> {
        self.database_name.as_deref()
    }
    fn set_database_name(&mut self, database_name: Option<String>) {
        self.database_name = database_name;
    }
}

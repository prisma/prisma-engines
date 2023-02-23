//! Composite types defined with the `type` keyword.

use crate::{default_value::DefaultValue, native_type_instance::NativeTypeInstance, scalars::ScalarType, FieldArity};
use psl_core::parser_database::ast;

#[derive(Debug, PartialEq, Clone)]
pub struct CompositeType {
    pub id: ast::CompositeTypeId,
    pub name: String,
    pub fields: Vec<CompositeTypeField>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct CompositeTypeField {
    pub name: String,
    pub r#type: CompositeTypeFieldType,
    pub arity: FieldArity,

    /// The database internal name.
    pub database_name: Option<String>,

    /// The default value of this field
    pub default_value: Option<DefaultValue>,
}

impl CompositeType {
    /// Gets an iterator over all scalar fields.
    pub fn scalar_fields(&self) -> impl Iterator<Item = &CompositeTypeField> {
        self.fields
            .iter()
            .filter(|f| matches!(f.r#type, CompositeTypeFieldType::Scalar(_, _)))
    }

    /// Gets an iterator over all enum fields.
    pub fn enum_fields(&self) -> impl Iterator<Item = &CompositeTypeField> {
        self.fields
            .iter()
            .filter(|f| matches!(f.r#type, CompositeTypeFieldType::Enum(_)))
    }

    /// Gets an iterator over all composite type fields.
    pub fn composite_type_fields(&self) -> impl Iterator<Item = &CompositeTypeField> {
        self.fields
            .iter()
            .filter(|f| matches!(f.r#type, CompositeTypeFieldType::CompositeType(_)))
    }

    /// Gets an iterator over all unsupported fields.
    pub fn unsupported_fields(&self) -> impl Iterator<Item = &CompositeTypeField> {
        self.fields
            .iter()
            .filter(|f| matches!(f.r#type, CompositeTypeFieldType::Unsupported(_)))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompositeTypeFieldType {
    CompositeType(String),
    Scalar(ScalarType, Option<NativeTypeInstance>),
    /// This is an enum field, with an enum of the given name.
    Enum(ast::EnumId),
    /// This is a field with an unsupported datatype. The content is the db's description of the type, it should enable migrate to create the type.
    Unsupported(String),
}

impl CompositeTypeFieldType {
    pub fn as_scalar(&self) -> Option<(&ScalarType, &Option<NativeTypeInstance>)> {
        if let Self::Scalar(typ, native_type) = self {
            Some((typ, native_type))
        } else {
            None
        }
    }
}

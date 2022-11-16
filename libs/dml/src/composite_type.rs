//! Composite types defined with the `type` keyword.

use crate::{
    default_value::DefaultValue, field::FieldArity, native_type_instance::NativeTypeInstance, scalars::ScalarType,
};

#[derive(Debug, PartialEq, Clone)]
pub struct CompositeType {
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

    /// Comments associated with this field.
    pub documentation: Option<String>,

    /// The default value of this field
    pub default_value: Option<DefaultValue>,

    /// Should we comment this field out.
    pub is_commented_out: bool,
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

    /// Finds a field by name.
    pub fn find_field(&self, name: &str) -> Option<&CompositeTypeField> {
        self.fields.iter().find(|f| f.name == name)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompositeTypeFieldType {
    CompositeType(String),
    Scalar(ScalarType, Option<NativeTypeInstance>),
    /// This is an enum field, with an enum of the given name.
    Enum(String),
    /// This is a field with an unsupported datatype. The content is the db's description of the type, it should enable migrate to create the type.
    Unsupported(String),
}

impl CompositeTypeFieldType {
    pub fn as_composite_type(&self) -> Option<&String> {
        if let Self::CompositeType(v) = self {
            Some(v)
        } else {
            None
        }
    }

    pub fn as_scalar(&self) -> Option<(&ScalarType, &Option<NativeTypeInstance>)> {
        if let Self::Scalar(typ, native_type) = self {
            Some((typ, native_type))
        } else {
            None
        }
    }

    pub fn as_native_type(&self) -> Option<(&ScalarType, &NativeTypeInstance)> {
        if let Self::Scalar(typ, Some(native_type)) = self {
            Some((typ, native_type))
        } else {
            None
        }
    }

    pub fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported(_))
    }
}

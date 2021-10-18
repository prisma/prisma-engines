//! Composite types defined with the `type` keyword.

use crate::{field::FieldArity, native_type_instance::NativeTypeInstance, scalars::ScalarType};

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
}

#[derive(Debug, PartialEq, Clone)]
pub enum CompositeTypeFieldType {
    CompositeType(String),
    /// The first option is Some(x) if the scalar type is based upon a type alias.
    Scalar(ScalarType, Option<String>, Option<NativeTypeInstance>),
}

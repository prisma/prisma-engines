use datamodel::{
    datamodel_connector::NativeTypeConstructor,
    diagnostics::{DatamodelError, Span},
    parser_database::ScalarType,
};
use native_types::MongoDbType;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// The names of types as they appear in the Prisma schema.
pub mod type_names {
    pub const STRING: &str = "String";
    pub const DOUBLE: &str = "Double";
    pub const LONG: &str = "Long";
    pub const INT: &str = "Int";
    pub const BIN_DATA: &str = "BinData";
    pub const OBJECT_ID: &str = "ObjectId";
    pub const BOOL: &str = "Bool";
    pub const DATE: &str = "Date";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const JSON: &str = "Json";
}

static DEFAULT_MAPPING: Lazy<HashMap<ScalarType, MongoDbType>> = Lazy::new(|| {
    vec![
        (ScalarType::Int, MongoDbType::Int),
        (ScalarType::BigInt, MongoDbType::Long),
        (ScalarType::Float, MongoDbType::Double),
        (ScalarType::Boolean, MongoDbType::Bool),
        (ScalarType::String, MongoDbType::String),
        (ScalarType::DateTime, MongoDbType::Timestamp),
        (ScalarType::Bytes, MongoDbType::BinData),
        (ScalarType::Json, MongoDbType::Json),
    ]
    .into_iter()
    .collect()
});

use type_names::*;

pub(crate) fn default_for(scalar_type: &ScalarType) -> &MongoDbType {
    DEFAULT_MAPPING
        .get(scalar_type)
        .unwrap_or_else(|| panic!("MongoDB native type mapping missing for '{:?}'", scalar_type))
}

pub(crate) const NATIVE_TYPE_CONSTRUCTORS: &[NativeTypeConstructor] = &[
    NativeTypeConstructor::without_args(STRING, &[ScalarType::String]),
    NativeTypeConstructor::without_args(DOUBLE, &[ScalarType::Float]),
    NativeTypeConstructor::without_args(LONG, &[ScalarType::Int, ScalarType::BigInt]),
    NativeTypeConstructor::without_args(INT, &[ScalarType::Int]),
    NativeTypeConstructor::without_args(BIN_DATA, &[ScalarType::Bytes]),
    NativeTypeConstructor::without_args(OBJECT_ID, &[ScalarType::String, ScalarType::Bytes]),
    NativeTypeConstructor::without_args(BOOL, &[ScalarType::Boolean]),
    NativeTypeConstructor::without_args(DATE, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(TIMESTAMP, &[ScalarType::DateTime]),
    NativeTypeConstructor::without_args(JSON, &[ScalarType::Json]),
];

pub(crate) fn mongo_type_from_input(name: &str, span: Span) -> Result<MongoDbType, DatamodelError> {
    let mongo_type = match name {
        STRING => MongoDbType::String,
        DOUBLE => MongoDbType::Double,
        LONG => MongoDbType::Long,
        INT => MongoDbType::Int,
        BIN_DATA => MongoDbType::BinData,
        OBJECT_ID => MongoDbType::ObjectId,
        BOOL => MongoDbType::Bool,
        DATE => MongoDbType::Date,
        TIMESTAMP => MongoDbType::Timestamp,
        JSON => MongoDbType::Json,
        name => return Err(DatamodelError::new_native_type_name_unknown("MongoDB", name, span)),
    };

    Ok(mongo_type)
}

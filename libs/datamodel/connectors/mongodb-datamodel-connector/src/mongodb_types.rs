use datamodel_connector::connector_error::{ConnectorError, ErrorKind};
use dml::{native_type_constructor::NativeTypeConstructor, scalars::ScalarType};
use native_types::MongoDbType;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// The names of types as they appear in the Prisma schema.
pub mod type_names {
    pub const STRING: &str = "String";
    pub const DOUBLE: &str = "Double";
    pub const LONG: &str = "Long";
    pub const INT: &str = "Int";
    pub const ARRAY: &str = "Array";
    pub const BIN_DATA: &str = "BinData";
    pub const OBJECT_ID: &str = "ObjectId";
    pub const BOOL: &str = "Bool";
    pub const DATE: &str = "Date";
    pub const TIMESTAMP: &str = "Timestamp";
    pub const DECIMAL: &str = "Decimal";
}

static DEFAULT_MAPPING: Lazy<HashMap<ScalarType, MongoDbType>> = Lazy::new(|| {
    vec![
        (ScalarType::Int, MongoDbType::Int),
        (ScalarType::BigInt, MongoDbType::Long),
        (ScalarType::Float, MongoDbType::Double),
        (ScalarType::Decimal, MongoDbType::Decimal),
        (ScalarType::Boolean, MongoDbType::Bool),
        (ScalarType::String, MongoDbType::String),
        (ScalarType::DateTime, MongoDbType::Timestamp),
        (ScalarType::Bytes, MongoDbType::BinData),
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

pub(crate) fn available_types() -> Vec<NativeTypeConstructor> {
    vec![
        NativeTypeConstructor::without_args(STRING, vec![ScalarType::String]),
        NativeTypeConstructor::without_args(DOUBLE, vec![ScalarType::Float]),
        NativeTypeConstructor::without_args(LONG, vec![ScalarType::Int, ScalarType::BigInt]),
        NativeTypeConstructor::without_args(INT, vec![ScalarType::Int]),
        NativeTypeConstructor::without_args(BIN_DATA, vec![ScalarType::Bytes]),
        NativeTypeConstructor::without_args(OBJECT_ID, vec![ScalarType::String, ScalarType::Bytes]),
        NativeTypeConstructor::without_args(BOOL, vec![ScalarType::Boolean]),
        NativeTypeConstructor::without_args(DATE, vec![ScalarType::DateTime]),
        NativeTypeConstructor::without_args(TIMESTAMP, vec![ScalarType::DateTime]),
        NativeTypeConstructor::without_args(DECIMAL, vec![ScalarType::Decimal]),
        NativeTypeConstructor::with_args(ARRAY, 1, all_types()),
    ]
}

pub(crate) fn mongo_type_from_input(name: &str, args: &[String]) -> crate::Result<MongoDbType> {
    let mongo_type = match name {
        STRING => MongoDbType::String,
        DOUBLE => MongoDbType::Double,
        LONG => MongoDbType::Long,
        INT => MongoDbType::Int,
        ARRAY => parse_array_type(args)?,
        BIN_DATA => MongoDbType::BinData,
        OBJECT_ID => MongoDbType::ObjectId,
        BOOL => MongoDbType::Bool,
        DATE => MongoDbType::Date,
        TIMESTAMP => MongoDbType::Timestamp,
        DECIMAL => MongoDbType::Decimal,
        name => {
            return Err(ConnectorError {
                kind: ErrorKind::NativeTypeNameUnknown {
                    connector_name: "MongoDB".to_owned(),
                    native_type: name.to_owned(),
                },
            })
        }
    };

    Ok(mongo_type)
}

fn all_types() -> Vec<ScalarType> {
    vec![
        ScalarType::Int,
        ScalarType::BigInt,
        ScalarType::Float,
        ScalarType::Boolean,
        ScalarType::String,
        ScalarType::DateTime,
        ScalarType::Json,
        ScalarType::Bytes,
        ScalarType::Decimal,
    ]
}

fn parse_array_type(args: &[String]) -> crate::Result<MongoDbType> {
    if args.len() != 1 {
        return Err(ConnectorError::new_argument_count_mismatch_error(ARRAY, 1, args.len()));
    }

    let type_arg = args.iter().next().unwrap();
    let inner_type = mongo_type_from_input(type_arg.as_str(), &[])?;

    Ok(MongoDbType::Array(Box::new(inner_type)))
}

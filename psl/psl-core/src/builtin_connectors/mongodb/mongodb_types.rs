use crate::parser_database::ScalarType;
use once_cell::sync::Lazy;
use std::collections::HashMap;

crate::native_type_definition! {
    /// MongoDB native types.
    /// Ignores deprecated and unsupported types for now.
    /// Taken from: <https://docs.mongodb.com/manual/reference/bson-types/>
    MongoDbType;
    String -> String,
    Double -> Float,
    BinData -> Bytes,
    ObjectId -> String | Bytes,
    Bool -> Boolean,
    Date -> DateTime,
    Int -> Int,
    Timestamp -> DateTime,
    Long -> Int | BigInt,
    Json -> Json,
    // Deprecated:
    // DbPointer
    // Undefined
    // Symbol

    // Unsupported:
    // Decimal,
    // MinKey,
    // MaxKey,
    // Object,
    // Javascript
    // JavascriptWithScope
    // Regex
}

static DEFAULT_MAPPING: Lazy<HashMap<ScalarType, MongoDbType>> = Lazy::new(|| {
    vec![
        (ScalarType::Int, MongoDbType::Long),
        (ScalarType::BigInt, MongoDbType::Long),
        (ScalarType::Float, MongoDbType::Double),
        (ScalarType::Boolean, MongoDbType::Bool),
        (ScalarType::String, MongoDbType::String),
        (ScalarType::DateTime, MongoDbType::Date),
        (ScalarType::Bytes, MongoDbType::BinData),
        (ScalarType::Json, MongoDbType::Json),
    ]
    .into_iter()
    .collect()
});

pub(crate) fn default_for(scalar_type: &ScalarType) -> &MongoDbType {
    DEFAULT_MAPPING
        .get(scalar_type)
        .unwrap_or_else(|| panic!("MongoDB native type mapping missing for '{scalar_type:?}'"))
}

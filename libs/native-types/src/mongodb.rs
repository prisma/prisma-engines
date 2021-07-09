use serde::*;
use serde_json::Value;

/// MongoDB native types.
/// Ignores deprecated and unsupported types for now.
/// Taken from: <https://docs.mongodb.com/manual/reference/bson-types/>
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MongoDbType {
    String,
    Double,
    Array(Box<MongoDbType>),
    BinData,
    ObjectId,
    Bool,
    Date,
    Int,
    Timestamp,
    Long,
    Decimal,
    // Deprecated:
    // DbPointer
    // Undefined
    // Symbol

    // Unsupported:
    // MinKey,
    // MaxKey,
    // Object,
    // Javascript
    // JavascriptWithScope
    // Regex
    // Null (as it's not a type but a value, despite being listed as one in Mongo)
}

impl super::NativeType for MongoDbType {
    fn to_json(&self) -> Value {
        serde_json::to_value(&self)
            .unwrap_or_else(|_| panic!("Serializing the native type to json failed: {:?}", &self))
    }
}

use datamodel::{NativeTypeInstance, ScalarType};
use mongodb::bson::Bson;
use native_types::MongoDbType;
use std::fmt;

use super::statistics::Name;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(super) enum FieldType {
    String,
    Double,
    BinData,
    ObjectId,
    Bool,
    Date,
    Int32,
    Timestamp,
    Int64,
    Decimal,
    Json,
    Document(String),
    Array(Box<FieldType>),
    Unsupported(&'static str),
}

impl FieldType {
    pub(super) fn from_bson(bson: &Bson, composite_name: Option<Name>) -> Option<Self> {
        match bson {
            Bson::Double(_) => Some(Self::Double),
            Bson::String(_) => Some(Self::String),
            Bson::Array(docs) if docs.is_empty() => None,
            Bson::Array(docs) => Some(Self::Array(Box::new(
                docs.first()
                    .and_then(|d| FieldType::from_bson(d, composite_name))
                    .unwrap_or(Self::Unsupported("Unknown")),
            ))),
            Bson::Document(_) => match composite_name {
                Some(name) => Some(Self::Document(name.take())),
                None => Some(Self::Json),
            },
            Bson::Boolean(_) => Some(Self::Bool),
            Bson::RegularExpression(_) => Some(Self::Unsupported("RegularExpression")),
            Bson::JavaScriptCode(_) => Some(Self::Unsupported("JavaScriptCode")),
            Bson::JavaScriptCodeWithScope(_) => Some(Self::Unsupported("JavaScriptCodeWithScope")),
            Bson::Int32(_) => Some(Self::Int32),
            Bson::Int64(_) => Some(Self::Int64),
            Bson::Timestamp(_) => Some(Self::Timestamp),
            Bson::Binary(_) => Some(Self::BinData),
            Bson::ObjectId(_) => Some(Self::ObjectId),
            Bson::DateTime(_) => Some(Self::Date),
            Bson::Symbol(_) => Some(Self::Unsupported("Symbol")),
            Bson::Decimal128(_) => Some(Self::Decimal),
            Bson::Undefined => Some(Self::Unsupported("Undefined")),
            Bson::MaxKey => Some(Self::Unsupported("MaxKey")),
            Bson::MinKey => Some(Self::Unsupported("MinKey")),
            Bson::DbPointer(_) => Some(Self::Unsupported("DbPointer")),
            Bson::Null => None,
        }
    }

    pub(super) fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::String => f.write_str("String"),
            FieldType::Double => f.write_str("Double"),
            FieldType::BinData => f.write_str("BinData"),
            FieldType::ObjectId => f.write_str("ObjectId"),
            FieldType::Bool => f.write_str("Boolean"),
            FieldType::Date => f.write_str("Date"),
            FieldType::Int32 => f.write_str("Int32"),
            FieldType::Timestamp => f.write_str("Timestamp"),
            FieldType::Int64 => f.write_str("Int64"),
            FieldType::Decimal => f.write_str("Decimal"),
            FieldType::Json => f.write_str("Document"),
            FieldType::Document(s) => f.write_str(s),
            FieldType::Array(r#type) => write!(f, "Array({})", r#type),
            FieldType::Unsupported(r#type) => write!(f, "{}", r#type),
        }
    }
}

impl From<FieldType> for datamodel::CompositeTypeFieldType {
    fn from(r#type: FieldType) -> Self {
        match r#type {
            FieldType::String => datamodel::CompositeTypeFieldType::Scalar(ScalarType::String, None, None),
            FieldType::Double => datamodel::CompositeTypeFieldType::Scalar(ScalarType::Float, None, None),
            FieldType::BinData => datamodel::CompositeTypeFieldType::Scalar(ScalarType::Bytes, None, None),
            FieldType::ObjectId => datamodel::CompositeTypeFieldType::Scalar(
                ScalarType::String,
                None,
                Some(NativeTypeInstance::new("ObjectId", Vec::new(), &MongoDbType::ObjectId)),
            ),
            FieldType::Bool => datamodel::CompositeTypeFieldType::Scalar(ScalarType::Boolean, None, None),
            FieldType::Date => datamodel::CompositeTypeFieldType::Scalar(
                ScalarType::DateTime,
                None,
                Some(NativeTypeInstance::new("Date", Vec::new(), &MongoDbType::Date)),
            ),
            FieldType::Int32 => datamodel::CompositeTypeFieldType::Scalar(ScalarType::Int, None, None),
            FieldType::Timestamp => datamodel::CompositeTypeFieldType::Scalar(ScalarType::DateTime, None, None),
            FieldType::Int64 => datamodel::CompositeTypeFieldType::Scalar(ScalarType::BigInt, None, None),
            FieldType::Decimal => datamodel::CompositeTypeFieldType::Scalar(ScalarType::Decimal, None, None),
            FieldType::Json => datamodel::CompositeTypeFieldType::Scalar(ScalarType::Json, None, None),
            FieldType::Document(name) => datamodel::CompositeTypeFieldType::CompositeType(name),
            FieldType::Array(r#type) => datamodel::CompositeTypeFieldType::from(*r#type),
            FieldType::Unsupported(name) => datamodel::CompositeTypeFieldType::Unsupported(name.to_string()),
        }
    }
}

impl From<FieldType> for datamodel::FieldType {
    fn from(r#type: FieldType) -> Self {
        match r#type {
            FieldType::String => datamodel::FieldType::Scalar(ScalarType::String, None, None),
            FieldType::Double => datamodel::FieldType::Scalar(ScalarType::Float, None, None),
            FieldType::BinData => datamodel::FieldType::Scalar(ScalarType::Bytes, None, None),
            FieldType::ObjectId => datamodel::FieldType::Scalar(
                ScalarType::String,
                None,
                Some(NativeTypeInstance::new("ObjectId", Vec::new(), &MongoDbType::ObjectId)),
            ),
            FieldType::Bool => datamodel::FieldType::Scalar(ScalarType::Boolean, None, None),
            FieldType::Date => datamodel::FieldType::Scalar(
                ScalarType::DateTime,
                None,
                Some(NativeTypeInstance::new("Date", Vec::new(), &MongoDbType::Date)),
            ),
            FieldType::Int32 => datamodel::FieldType::Scalar(ScalarType::Int, None, None),
            FieldType::Timestamp => datamodel::FieldType::Scalar(ScalarType::DateTime, None, None),
            FieldType::Int64 => datamodel::FieldType::Scalar(ScalarType::BigInt, None, None),
            FieldType::Decimal => datamodel::FieldType::Scalar(ScalarType::Decimal, None, None),
            FieldType::Json => datamodel::FieldType::Scalar(ScalarType::Json, None, None),
            FieldType::Document(name) => datamodel::FieldType::CompositeType(name),
            FieldType::Array(r#type) => datamodel::FieldType::from(*r#type),
            FieldType::Unsupported(type_name) => datamodel::FieldType::Unsupported(type_name.to_string()),
        }
    }
}

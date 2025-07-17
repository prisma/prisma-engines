use super::statistics::Name;
use bson::Bson;
use std::fmt;

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
            Bson::Decimal128(_) => Some(Self::Unsupported("Decimal128")),
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

    pub(super) fn is_unsupported(&self) -> bool {
        matches!(self, Self::Unsupported(_))
    }

    pub(super) fn is_document(&self) -> bool {
        matches!(self, Self::Document(_))
    }

    pub(super) fn has_documents(&self) -> bool {
        match self {
            Self::Document(_) | Self::Json => true,
            Self::Array(typ) => typ.is_document(),
            _ => false,
        }
    }

    pub(super) fn prisma_type(&self) -> &str {
        match self {
            FieldType::String => "String",
            FieldType::Double => "Float",
            FieldType::BinData => "Bytes",
            FieldType::ObjectId => "String",
            FieldType::Bool => "Boolean",
            FieldType::Date => "DateTime",
            FieldType::Int32 => "Int",
            FieldType::Timestamp => "DateTime",
            FieldType::Int64 => "BigInt",
            FieldType::Json => "Json",
            FieldType::Document(s) => s,
            FieldType::Array(r#type) => r#type.prisma_type(),
            FieldType::Unsupported(r#type) => r#type,
        }
    }

    pub(super) fn native_type(&self) -> Option<&str> {
        match self {
            Self::ObjectId => Some("ObjectId"),
            FieldType::Date => Some("Date"),
            _ => None,
        }
    }
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FieldType::String => f.write_str("String"),
            FieldType::Double => f.write_str("Float"),
            FieldType::BinData => f.write_str("Binary"),
            FieldType::ObjectId => f.write_str("String (ObjectId)"),
            FieldType::Bool => f.write_str("Boolean"),
            FieldType::Date => f.write_str("DateTime (Date)"),
            FieldType::Int32 => f.write_str("Int"),
            FieldType::Timestamp => f.write_str("DateTime (Timestamp)"),
            FieldType::Int64 => f.write_str("BigInt"),
            FieldType::Json => f.write_str("Json"),
            FieldType::Document(s) => f.write_str(s),
            FieldType::Array(r#type) => write!(f, "Array({type})"),
            FieldType::Unsupported(r#type) => write!(f, "{type}"),
        }
    }
}

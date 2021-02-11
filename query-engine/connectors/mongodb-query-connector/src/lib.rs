mod error;
mod filter;
mod interface;
mod projection;
mod queries;
mod value;

use error::MongoError;
pub use interface::*;
use mongodb::bson::{Bson, Document};
use prisma_models::{ScalarFieldRef, TypeIdentifier};

type Result<T> = std::result::Result<T, MongoError>;

trait IntoBson {
    fn into_bson(self) -> Result<Bson>;
}

trait BsonTransform {
    fn into_document(self) -> Result<Document>;
}

impl BsonTransform for Bson {
    fn into_document(self) -> Result<Document> {
        if let Bson::Document(doc) = self {
            Ok(doc)
        } else {
            Err(MongoError::ConversionError {
                from: format!("{:?}", self),
                to: "Bson::Document".to_string(),
            })
        }
    }
}

/// Best guess: If a field is String typed, is an ID and has a dbgenerated value, we assume it's an ObjectID.
pub(crate) fn guess_is_object_id_field(field: &ScalarFieldRef) -> bool {
    let is_string_field = matches!(field.type_identifier, TypeIdentifier::String);
    let is_id = field.is_id;

    field
        .default_value
        .as_ref()
        .map(|dv| dv.is_dbgenerated() && is_string_field && is_id)
        .unwrap_or(false)
}

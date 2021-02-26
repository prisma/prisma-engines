mod cursor;
mod error;
mod filter;
mod interface;
mod join;
mod orderby;
mod projection;
mod queries;
mod query_arguments;
mod value;

use error::MongoError;
pub use interface::*;
use mongodb::bson::{Bson, Document};

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

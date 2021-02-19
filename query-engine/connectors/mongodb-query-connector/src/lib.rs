mod error;
mod filter;
mod interface;
mod projection;
mod queries;
mod query_arguments;
mod value;

use error::MongoError;
pub use interface::*;
use mongodb::bson::{Bson, Document};
use prisma_models::RelationFieldRef;
use tracing::log::kv::source;

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

#[derive(Debug)]
pub(crate) struct JoinStage {
    pub(crate) source_field: RelationFieldRef,
    pub(crate) document: Document,
}

impl JoinStage {
    pub(crate) fn new(source_field: RelationFieldRef, document: Document) -> Self {
        Self { source_field, document }
    }
}

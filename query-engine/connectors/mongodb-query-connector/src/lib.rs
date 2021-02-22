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

/// A Mongo Join stage can in itself contain joins that traverse relations.
/// Every hop made (in left-to-right order) is contained here as the respective source relation field.
#[derive(Debug)]
pub(crate) struct JoinStage {
    /// The starting point of the traversal (left model of the join).
    pub(crate) source: RelationFieldRef,

    /// Nested joins
    pub(crate) nested: Vec<JoinStage>,
}

impl JoinStage {
    pub(crate) fn new(source: RelationFieldRef) -> Self {
        Self { source, nested: vec![] }
    }

    pub(crate) fn add_nested(&mut self, stage: JoinStage) {
        self.nested.push(stage);
    }

    pub(crate) fn build(self) -> Document {
        todo!()
    }
}

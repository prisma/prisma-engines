mod error;
mod filter;
mod interface;
mod projection;
mod queries;

pub use interface::*;
use mongodb::bson::Document;

type Result<T> = std::result::Result<T, error::MongoError>;

trait IntoBsonDocument {
    fn into_bson(&self) -> Result<Document>;
}

use connector_interface::Filter;
use mongodb::bson::Document;

use crate::IntoBsonDocument;

// Mongo filters are a BSON document.
impl IntoBsonDocument for Filter {
    fn into_bson(&self) -> crate::Result<Document> {
        let doc = Document::new();

        //

        Ok(doc)
    }
}

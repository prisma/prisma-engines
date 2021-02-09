use mongodb::bson::{Bson, Document};
use prisma_models::ModelProjection;

use crate::IntoBsonDocument;

impl IntoBsonDocument for ModelProjection {
    fn into_bson(&self) -> crate::Result<Document> {
        let mut doc = Document::new();

        for field in self.scalar_fields() {
            doc.insert(field.db_name(), Bson::Boolean(true)); // Or maybe "include" string?
        }

        Ok(doc)
    }
}

use crate::IntoBson;
use mongodb::bson::{Bson, Document};
use prisma_models::ModelProjection;

// Used as projection document for Mongo queries.
impl IntoBson for ModelProjection {
    fn into_bson(self) -> crate::Result<Bson> {
        let mut doc = Document::new();

        for field in self.scalar_fields() {
            doc.insert(field.db_name(), Bson::Int32(1));
        }

        Ok(doc.into())
    }
}

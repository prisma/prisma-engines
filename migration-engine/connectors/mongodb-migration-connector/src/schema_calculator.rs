use crate::schema::MongoSchema;
use bson::{Bson, Document};
use datamodel::Datamodel;

/// Datamodel -> MongoSchema
pub(crate) fn calculate(datamodel: &Datamodel) -> MongoSchema {
    let mut schema = MongoSchema::default();

    for model in datamodel.models() {
        let collection_id = schema.push_collection(model.database_name.as_ref().unwrap_or(&model.name).clone());

        for index in &model.indices {
            let name = index.db_name.clone().expect("unnamed index");
            let mut path = Document::new();
            let fields = index
                .fields
                .iter()
                .map(|field_name| model.find_scalar_field(field_name).unwrap().db_name())
                .map(|field_final_name: &str| (field_final_name.to_owned(), Bson::Int32(1)));

            path.extend(fields);

            schema.push_index(collection_id, name, index.is_unique(), path);
        }
    }

    schema
}

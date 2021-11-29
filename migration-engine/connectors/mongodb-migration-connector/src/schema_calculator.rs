use crate::schema::MongoSchema;
use datamodel::Datamodel;
use mongodb::bson::{Bson, Document};

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
                .map(|field| {
                    let sf = model.find_scalar_field(&field.name).unwrap();
                    (sf.db_name(), field.sort_order)
                })
                .map(|(name, sort_order)| {
                    (
                        name.to_owned(),
                        match sort_order {
                            Some(datamodel::SortOrder::Desc) => Bson::Int32(-1),
                            None if index.is_fulltext() => Bson::String("text".to_string()),
                            _ => Bson::Int32(1),
                        },
                    )
                });

            path.extend(fields);

            schema.push_index(collection_id, name, index.tpe, path);
        }
    }

    schema
}

mod field_type;
mod statistics;

use datamodel::Datamodel;
use futures::TryStreamExt;
use introspection_connector::{IntrospectionResult, Version};
use mongodb::{
    bson::{doc, Document},
    Database,
};
use statistics::*;

pub(super) async fn sample(database: Database) -> crate::Result<IntrospectionResult> {
    let collections = database.list_collection_names(None).await?;
    let mut statistics = Statistics::default();

    for collection_name in collections {
        let collection = database.collection::<Document>(&collection_name);

        let mut documents = collection
            .aggregate(vec![doc! { "$sample": { "size": 10000 } }], None)
            .await?;

        while let Some(document) = documents.try_next().await? {
            statistics.track_document_types(&collection_name, document);
        }

        let mut indices = collection.list_indexes(None).await?;

        while let Some(index) = indices.try_next().await? {
            statistics.track_index(&collection_name, index);
        }
    }

    Ok(IntrospectionResult {
        data_model: Datamodel::from(statistics),
        warnings: Vec::new(),
        version: Version::NonPrisma,
    })
}

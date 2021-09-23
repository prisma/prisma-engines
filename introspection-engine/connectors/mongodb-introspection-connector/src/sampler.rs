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

        let mut cursor = collection
            .aggregate(vec![doc! { "$sample": { "size": 10000 } }], None)
            .await?;

        while let Some(document) = cursor.try_next().await? {
            statistics.track(&collection_name, document);
        }
    }

    Ok(IntrospectionResult {
        data_model: Datamodel::from(statistics),
        warnings: Vec::new(),
        version: Version::NonPrisma,
    })
}

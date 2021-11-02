mod field_type;
mod statistics;

use futures::TryStreamExt;
use introspection_connector::{CompositeTypeDepth, IntrospectionResult, Version};
use mongodb::{
    bson::{doc, Document},
    options::AggregateOptions,
    Database,
};
use statistics::*;

/// From the given database, lists all collections as models, and samples
/// maximum of SAMPLE_SIZE documents for their fields with the following rules:
///
/// - If the same field differs in types between documents, takes the most
/// common type or if even, the latest type and adds a warning.
/// - Missing fields count as null.
/// - Indices are taken, but not if they are partial.
pub(super) async fn sample(
    database: Database,
    composite_type_depth: CompositeTypeDepth,
) -> crate::Result<IntrospectionResult> {
    let collections = database.list_collection_names(None).await?;
    let mut statistics = Statistics::new(composite_type_depth);
    let mut warnings = Vec::new();

    for collection_name in &collections {
        statistics.track_model(collection_name);
    }

    for collection_name in collections {
        let collection = database.collection::<Document>(&collection_name);

        let options = AggregateOptions::builder().allow_disk_use(Some(true)).build();

        let mut documents = collection
            .aggregate(vec![doc! { "$sample": { "size": SAMPLE_SIZE } }], Some(options))
            .await?;

        while let Some(document) = documents.try_next().await? {
            statistics.track_model_fields(&collection_name, document);
        }

        let mut indices = collection.list_indexes(None).await?;

        while let Some(index) = indices.try_next().await? {
            statistics.track_index(&collection_name, index);
        }
    }

    let data_model = statistics.into_datamodel(&mut warnings);

    Ok(IntrospectionResult {
        data_model,
        warnings,
        version: Version::NonPrisma,
    })
}

mod field_type;
mod statistics;

use futures::TryStreamExt;
use introspection_connector::{CompositeTypeDepth, IntrospectionResult, Version};
use mongodb::{
    bson::{doc, Document},
    options::AggregateOptions,
    Database,
};
use mongodb_schema_describer::MongoSchema;
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
    schema: MongoSchema,
) -> crate::Result<IntrospectionResult> {
    let mut statistics = Statistics::new(composite_type_depth);
    let mut warnings = Vec::new();

    for collection in schema.walk_collections() {
        statistics.track_model(collection.name());
    }

    for collection in schema.walk_collections() {
        let options = AggregateOptions::builder().allow_disk_use(Some(true)).build();

        let mut documents = database
            .collection::<Document>(collection.name())
            .aggregate(vec![doc! { "$sample": { "size": SAMPLE_SIZE } }], Some(options))
            .await?;

        while let Some(document) = documents.try_next().await? {
            statistics.track_model_fields(collection.name(), document);
        }

        for index in collection.indexes() {
            statistics.track_index(collection.name(), index);
        }
    }

    let data_model = statistics.into_datamodel(&mut warnings);

    Ok(IntrospectionResult {
        data_model,
        warnings,
        version: Version::NonPrisma,
    })
}

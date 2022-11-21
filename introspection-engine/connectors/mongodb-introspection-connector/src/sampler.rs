mod field_type;
mod statistics;

use datamodel_renderer as render;
use futures::TryStreamExt;
use introspection_connector::{CompositeTypeDepth, IntrospectionContext, IntrospectionResult, Version};
use mongodb::{
    bson::{doc, Document},
    options::AggregateOptions,
    Database,
};
use mongodb_schema_describer::MongoSchema;
pub(crate) use statistics::Name;
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
    schema: MongoSchema,
    ctx: &IntrospectionContext,
) -> crate::Result<IntrospectionResult> {
    let mut statistics = Statistics::new(ctx.composite_type_depth);
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
    let is_empty = data_model.is_empty();

    let mut rendered = render::Datamodel::default();
    rendered.push_dml(ctx.datasource(), &data_model);
    let config = if ctx.render_config {
        render::Configuration::from_psl(ctx.configuration()).to_string()
    } else {
        String::new()
    };

    let data_model = format!("{config}\n{rendered}");

    Ok(IntrospectionResult {
        data_model: psl::reformat(&data_model, 2).unwrap(),
        is_empty,
        warnings,
        version: Version::NonPrisma,
    })
}

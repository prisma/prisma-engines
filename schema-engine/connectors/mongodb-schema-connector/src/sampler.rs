mod field_type;
mod statistics;

use datamodel_renderer as render;
use futures::TryStreamExt;
use mongodb::{
    bson::{doc, Document},
    options::AggregateOptions,
    Database,
};
use mongodb_schema_describer::MongoSchema;
use schema_connector::{warnings::Model, IntrospectionContext, IntrospectionResult, Warnings};
use statistics::*;
use std::borrow::Cow;

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
) -> Result<IntrospectionResult, mongodb::error::Error> {
    let mut statistics = Statistics::new(ctx.composite_type_depth);
    let mut warnings = Warnings::new();

    for collection in schema.walk_collections() {
        statistics.track_model(collection.name(), collection);

        if collection.has_schema() {
            warnings.json_schema_defined.push(Model {
                model: collection.name().to_owned(),
            })
        }

        if collection.is_capped() {
            warnings.capped_collection.push(Model {
                model: collection.name().to_owned(),
            })
        }
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

    let mut data_model = render::Datamodel::default();

    // Ensures that all previous files are present in the new datamodel, even when empty after re-introspection.
    for file_id in ctx.previous_schema().db.iter_file_ids() {
        let file_name = ctx.previous_schema().db.file_name(file_id);

        data_model.create_empty_file(Cow::Borrowed(file_name));
    }

    statistics.render(ctx, &mut data_model, &mut warnings);

    let is_empty = data_model.is_empty();

    if ctx.render_config {
        let config = render::Configuration::from_psl(ctx.configuration(), ctx.previous_schema(), None);

        data_model.set_configuration(config);
    }

    let sources = data_model.render();

    let warnings = if !warnings.is_empty() {
        Some(warnings.to_string())
    } else {
        None
    };

    Ok(IntrospectionResult {
        datamodels: psl::reformat_multiple(sources, 2),
        is_empty,
        warnings,
        views: None,
    })
}

//! Tooling to go from PSL and database schema to a PSL string.

mod configuration;
mod defaults;
mod enums;
mod id;
mod indexes;
mod models;
mod postgres;
mod relation_field;
mod scalar_field;
mod views;

use crate::introspection::datamodel_calculator::DatamodelCalculatorContext;
use datamodel_renderer as renderer;
use psl::PreviewFeature;
use schema_connector::ViewDefinition;
use std::borrow::Cow;

/// Combines the SQL database schema and an existing PSL schema to a
/// PSL schema definition string.
pub(crate) fn to_psl_string(
    introspection_file_name: Cow<'_, str>,
    ctx: &DatamodelCalculatorContext<'_>,
) -> (Vec<(String, String)>, bool, Vec<ViewDefinition>) {
    let mut datamodel = renderer::Datamodel::new();
    let mut views = Vec::new();

    // Ensures that all previous files are present in the new datamodel, even when empty after re-introspection.
    for file_id in ctx.previous_schema.db.iter_file_ids() {
        let file_name = ctx.previous_schema.db.file_name(file_id);

        datamodel.create_empty_file(Cow::Borrowed(file_name));
    }

    enums::render(introspection_file_name.clone(), ctx, &mut datamodel);
    models::render(introspection_file_name.clone(), ctx, &mut datamodel);

    if ctx.config.preview_features().contains(PreviewFeature::Views) {
        views.extend(views::render(introspection_file_name, ctx, &mut datamodel));
    }

    let is_empty = datamodel.is_empty();

    if ctx.render_config {
        let config = configuration::render(ctx.previous_schema, ctx.sql_schema, ctx.force_namespaces);

        datamodel.set_configuration(config);
    }

    let sources = datamodel.render();

    (psl::reformat_multiple(sources, 2), is_empty, views)
}

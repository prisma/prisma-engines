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

/// Combines the SQL database schema and an existing PSL schema to a
/// PSL schema definition string.
pub(crate) fn to_psl_string(
    introspection_file_name: &str,
    ctx: &DatamodelCalculatorContext<'_>,
) -> (Vec<(String, String)>, bool, Vec<ViewDefinition>) {
    let mut datamodel = renderer::Datamodel::new();
    let mut views = Vec::new();

    enums::render(introspection_file_name, ctx, &mut datamodel);
    models::render(introspection_file_name, ctx, &mut datamodel);

    if ctx.config.preview_features().contains(PreviewFeature::Views) {
        views.extend(views::render(introspection_file_name, ctx, &mut datamodel));
    }

    let is_empty = datamodel.is_empty();

    if ctx.render_config {
        let config = configuration::render(ctx.previous_schema, ctx.sql_schema, ctx.force_namespaces);

        datamodel.set_configuration(config);
    }

    let sources = datamodel.render();

    dbg!(&sources);

    (psl::reformat_multiple(sources, 2), is_empty, views)
}

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
pub(crate) fn to_psl_string(ctx: &DatamodelCalculatorContext<'_>) -> (String, bool, Vec<ViewDefinition>) {
    let mut rendered = renderer::Datamodel::new();
    let mut views = Vec::new();

    enums::render(ctx, &mut rendered);
    models::render(ctx, &mut rendered);

    if ctx.config.preview_features().contains(PreviewFeature::Views) {
        views.extend(views::render(ctx, &mut rendered));
    }

    let psl_string = if ctx.render_config {
        let config = configuration::render(ctx.config, ctx.sql_schema, ctx.force_namespaces);
        format!("{config}\n{rendered}")
    } else {
        rendered.to_string()
    };

    (psl::reformat(&psl_string, 2).unwrap(), rendered.is_empty(), views)
}

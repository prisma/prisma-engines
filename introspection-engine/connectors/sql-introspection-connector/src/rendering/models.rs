//! Rendering of model blocks.

use super::{id, indexes, relation_field, scalar_field};
use crate::{
    datamodel_calculator::DatamodelCalculatorContext,
    introspection_helpers::{self as helpers, compare_options_none_last},
    pair::ModelPair,
};
use datamodel_renderer::datamodel as renderer;
use quaint::prelude::SqlFamily;

/// Render all model blocks to the PSL.
pub(super) fn render<'a>(ctx: &'a DatamodelCalculatorContext<'a>, rendered: &mut renderer::Datamodel<'a>) {
    let mut models_with_idx: Vec<(Option<_>, renderer::Model<'a>)> = Vec::with_capacity(ctx.sql_schema.tables_count());

    for model in ctx.model_pairs() {
        models_with_idx.push((model.previous_position(), render_model(model, ctx.sql_family)));
    }

    models_with_idx.sort_by(|(a, _), (b, _)| helpers::compare_options_none_last(*a, *b));

    for (_, render) in models_with_idx.into_iter() {
        rendered.push_model(render);
    }
}

/// Render a single model.
fn render_model(model: ModelPair<'_>, sql_family: SqlFamily) -> renderer::Model<'_> {
    let mut rendered = renderer::Model::new(model.name());

    if let Some(docs) = model.documentation() {
        rendered.documentation(docs);
    }

    if let Some(mapped_name) = model.mapped_name() {
        rendered.map(mapped_name);

        if model.uses_reserved_name() {
            let docs = format!(
                "This model has been renamed to '{}' during introspection, because the original name '{}' is reserved.",
                model.name(),
                mapped_name,
            );

            rendered.documentation(docs);
        }
    }

    if model.is_partition() {
        let docs = "This table is a partition table and requires additional setup for migrations. Visit https://pris.ly/d/partition-tables for more info.";

        rendered.documentation(docs);
    }

    if let Some(namespace) = model.namespace() {
        rendered.schema(namespace);
    }

    if model.ignored() {
        rendered.ignore();
    }

    if let Some(id) = model.id() {
        rendered.id(id::render(id));
    }

    if model.scalar_fields().len() == 0 {
        // On postgres this is allowed, on the other dbs, this could be a symptom of missing privileges.
        let docs = if sql_family.is_postgres() {
            "We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges."
        } else {
            "We could not retrieve columns for the underlying table. You probably have no rights to see them. Please check your privileges."
        };

        rendered.documentation(docs);
        rendered.comment_out();
    } else if !model.has_usable_identifier() && !model.ignored_in_psl() {
        let docs = "The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.";

        rendered.documentation(docs);
    }

    for field in model.scalar_fields() {
        rendered.push_field(scalar_field::render(field));
    }

    for field in model.relation_fields() {
        rendered.push_field(relation_field::render(field));
    }

    let mut ordered_indexes: Vec<_> = model
        .indexes()
        .map(|idx| (idx.previous_position(), indexes::render(idx)))
        .collect();

    ordered_indexes.sort_by(|(idx, _), (idx_b, _)| compare_options_none_last(*idx, *idx_b));

    for (_, definition) in ordered_indexes {
        rendered.push_index(definition);
    }

    rendered
}

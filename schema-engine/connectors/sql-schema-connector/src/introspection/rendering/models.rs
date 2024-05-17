//! Rendering of model blocks.

use std::borrow::Cow;

use super::{id, indexes, relation_field, scalar_field};
use crate::introspection::{
    datamodel_calculator::DatamodelCalculatorContext,
    introspection_helpers::{self as helpers, compare_options_none_last},
    introspection_pair::ModelPair,
};
use datamodel_renderer::datamodel as renderer;
use quaint::prelude::SqlFamily;

/// Render all model blocks to the PSL.
pub(super) fn render<'a>(
    introspection_file_name: &'a str,
    ctx: &'a DatamodelCalculatorContext<'a>,
    rendered: &mut renderer::Datamodel<'a>,
) {
    let mut models_with_idx: Vec<(Option<_>, renderer::Model<'a>)> = Vec::with_capacity(ctx.sql_schema.tables_count());

    for model in ctx.model_pairs() {
        models_with_idx.push((model.previous_position(), render_model(model, ctx.sql_family)));
    }

    models_with_idx.sort_by(|(a, _), (b, _)| helpers::compare_options_none_last(*a, *b));

    for (previous_model, render) in models_with_idx.into_iter() {
        let file_name = match previous_model {
            Some((prev_file_id, _)) => ctx.previous_schema.db.file_name(prev_file_id),
            None => introspection_file_name,
        };

        rendered.push_model(Cow::Borrowed(file_name), render);
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

    if model.new_with_partition() {
        let docs = "This table is a partition table and requires additional setup for migrations. Visit https://pris.ly/d/partition-tables for more info.";

        rendered.documentation(docs);
    }

    if model.new_with_subclass() {
        let docs = "This table has subclasses and requires additional setup for migrations. Visit https://pris.ly/d/table-inheritance for more info.";

        rendered.documentation(docs);
    }

    if model.adds_check_constraints() {
        let docs = "This table contains check constraints and requires additional setup for migrations. Visit https://pris.ly/d/check-constraints for more info.";

        rendered.documentation(docs);
    }

    if model.adds_exclusion_constraints() {
        let docs = "This table contains exclusion constraints and requires additional setup for migrations. Visit https://pris.ly/d/exclusion-constraints for more info.";

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
        let docs = "The underlying table does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.";

        rendered.documentation(docs);
    }

    if model.adds_a_description() {
        let docs = "This model or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments";
        rendered.documentation(docs);
    }

    if model.adds_a_row_level_ttl() {
        let docs = "This model is using a row level TTL in the database, and requires an additional setup in migrations. Read more: https://pris.ly/d/row-level-ttl";

        rendered.documentation(docs);
    }

    if model.adds_non_default_deferring() {
        let docs = "This model has constraints using non-default deferring rules and requires additional setup for migrations. Visit https://pris.ly/d/constraint-deferring for more info.";

        rendered.documentation(docs);
    }

    if model.adds_row_level_security() {
        let docs= "This model contains row level security and requires additional setup for migrations. Visit https://pris.ly/d/row-level-security for more info.";

        rendered.documentation(docs);
    }

    if model.adds_non_default_null_position() {
        let docs = "This model contains an index with non-default null sort order and requires additional setup for migrations. Visit https://pris.ly/d/default-index-null-ordering for more info.";

        rendered.documentation(docs);
    }

    if model.expression_indexes().next().is_some() {
        let docs = "This model contains an expression index which requires additional setup for migrations. Visit https://pris.ly/d/expression-indexes for more info.";

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

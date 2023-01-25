//! Rendering of model blocks.

use super::{id, relation_field, scalar_field};
use crate::{
    datamodel_calculator::{InputContext, OutputContext},
    introspection_helpers::{self as helpers, compare_options_none_last},
    pair::ModelPair,
    warnings::{self, Warnings},
};
use datamodel_renderer::datamodel as renderer;

use super::indexes;

/// Render all model blocks to the PSL.
pub(super) fn render<'a>(input: InputContext<'a>, output: &mut OutputContext<'a>) {
    let mut models_with_idx: Vec<(Option<_>, renderer::Model<'a>)> = Vec::with_capacity(input.schema.tables_count());

    for model in input.model_pairs() {
        models_with_idx.push((
            model.previous_position(),
            render_model(model, input, &mut output.warnings),
        ));
    }

    models_with_idx.sort_by(|(a, _), (b, _)| helpers::compare_options_none_last(*a, *b));

    for (_, render) in models_with_idx.into_iter() {
        output.rendered_schema.push_model(render);
    }
}

/// Render a single model.
fn render_model<'a>(model: ModelPair<'a>, input: InputContext<'a>, warnings: &mut Warnings) -> renderer::Model<'a> {
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

    if let Some(namespace) = model.namespace() {
        rendered.schema(namespace);
    }

    if model.ignored() {
        rendered.ignore();
    }

    if let Some(id) = model.id() {
        rendered.id(id::render(id, warnings));
    }

    if model.scalar_fields().len() == 0 {
        rendered.documentation(empty_table_comment(input));
        rendered.comment_out();

        warnings.models_without_columns.push(warnings::Model {
            model: model.name().to_string(),
        });
    } else if !model.has_usable_identifier() {
        let docs = "The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.";

        rendered.documentation(docs);

        warnings.models_without_identifiers.push(warnings::Model {
            model: model.name().to_string(),
        });
    }

    if model.uses_duplicate_name() {
        warnings.duplicate_names.push(warnings::TopLevelItem {
            r#type: warnings::TopLevelType::Model,
            name: model.name().to_string(),
        })
    }

    if model.remapped_name() {
        warnings.remapped_models.push(warnings::Model {
            model: model.name().to_string(),
        });
    }

    for field in model.scalar_fields() {
        rendered.push_field(scalar_field::render(field, warnings));
    }

    for field in model.relation_fields() {
        rendered.push_field(relation_field::render(field, warnings));
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

fn empty_table_comment(input: InputContext<'_>) -> &'static str {
    // On postgres this is allowed, on the other dbs, this could be a symptom of missing privileges.
    if input.sql_family.is_postgres() {
        "We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges."
    } else {
        "We could not retrieve columns for the underlying table. You probably have no rights to see them. Please check your privileges."
    }
}

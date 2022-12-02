use super::scalar_field;
use crate::{
    calculate_datamodel::{InputContext, OutputContext},
    introspection_helpers as helpers, warnings,
};
use datamodel_renderer::datamodel as renderer;
use sql_schema_describer as sql;

use super::indexes;

pub(super) fn render<'a>(input: InputContext<'a>, output: &mut OutputContext<'a>) {
    let mut models_with_idx: Vec<(Option<_>, sql::TableId, renderer::Model<'a>)> =
        Vec::with_capacity(input.schema.tables_count());

    for model in input.model_pairs() {
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
            let fields = id.fields().map(|field| {
                let mut rendered = renderer::IndexFieldInput::new(field.name());

                if let Some(sort_order) = field.sort_order() {
                    rendered.sort_order(sort_order);
                }

                if let Some(length) = field.length() {
                    rendered.length(length);
                }

                rendered
            });

            let mut definition = renderer::IdDefinition::new(fields);

            if let Some(name) = id.name() {
                definition.name(name);

                output.warnings.reintrospected_id_names.push(warnings::Model {
                    model: model.name().to_string(),
                });
            }

            if let Some(map) = id.mapped_name() {
                definition.map(map);
            }

            if let Some(clustered) = id.clustered() {
                definition.clustered(clustered);
            }

            rendered.id(definition);
        }

        if model.scalar_fields().len() == 0 {
            rendered.documentation(empty_table_comment(input));
            rendered.comment_out();

            output.warnings.models_without_columns.push(warnings::Model {
                model: model.name().to_string(),
            });
        } else if !model.has_usable_identifier() {
            let docs = "The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.";

            rendered.documentation(docs);

            output.warnings.models_without_identifiers.push(warnings::Model {
                model: model.name().to_string(),
            });
        }

        if model.remapped_name() {
            output.warnings.remapped_models.push(warnings::Model {
                model: model.name().to_string(),
            });
        }

        for field in model.scalar_fields() {
            rendered.push_field(scalar_field::render(field, output));
        }

        indexes::render(model, &mut rendered);

        models_with_idx.push((model.previous_position(), model.table_id(), rendered));
    }

    models_with_idx.sort_by(|(a, _, _), (b, _, _)| helpers::compare_options_none_last(*a, *b));

    for (idx, (_, table_id, render)) in models_with_idx.into_iter().enumerate() {
        output.rendered_schema.push_model(render);
        output.target_models.insert(table_id, idx);
    }
}

fn empty_table_comment(input: InputContext<'_>) -> &'static str {
    // On postgres this is allowed, on the other dbs, this could be a symptom of missing privileges.
    if input.sql_family.is_postgres() {
        "We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges."
    } else {
        "We could not retrieve columns for the underlying table. You probably have no rights to see them. Please check your privileges."
    }
}

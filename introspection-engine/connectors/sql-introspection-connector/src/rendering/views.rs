use crate::{
    datamodel_calculator::{InputContext, OutputContext},
    introspection_helpers as helpers,
    pair::ViewPair,
    warnings::{self, Warnings},
};
use datamodel_renderer::datamodel as renderer;
use psl::parser_database::{walkers, SortOrder};

use super::scalar_field;

/// Render all view blocks to the PSL.
pub(super) fn render<'a>(input: InputContext<'a>, output: &mut OutputContext<'a>) {
    let mut views_with_idx: Vec<(Option<_>, renderer::View<'a>)> = Vec::with_capacity(input.schema.views_count());

    for view in input.view_pairs() {
        views_with_idx.push((view.previous_position(), render_view(view, input, &mut output.warnings)));
    }

    views_with_idx.sort_by(|(a, _), (b, _)| helpers::compare_options_none_last(*a, *b));

    for (_, render) in views_with_idx.into_iter() {
        output.rendered_schema.push_view(render);
    }
}

/// Render a single view.
fn render_view<'a>(view: ViewPair<'a>, input: InputContext<'a>, warnings: &mut Warnings) -> renderer::View<'a> {
    let mut rendered = renderer::View::new(view.name());

    if let Some(docs) = view.documentation() {
        rendered.documentation(docs);
    }

    if let Some(mapped_name) = view.mapped_name() {
        rendered.map(mapped_name);

        if view.uses_reserved_name() {
            let docs = format!(
                "This view has been renamed to '{}' during introspection, because the original name '{}' is reserved.",
                view.name(),
                mapped_name,
            );

            rendered.documentation(docs);
        }
    }

    if let Some(namespace) = view.namespace() {
        rendered.schema(namespace);
    }

    if view.ignored() {
        rendered.ignore();
    }

    if let Some(id) = view.id() {
        rendered.id(render_id(id));
    }

    if !view.has_usable_identifier() {
        let docs = "The underlying view does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.";
        rendered.documentation(docs);

        warnings.views_without_identifiers.push(warnings::View {
            view: view.name().to_string(),
        });
    }

    if view.uses_duplicate_name() {
        warnings.duplicate_names.push(warnings::TopLevelItem {
            r#type: warnings::TopLevelType::View,
            name: view.name().to_string(),
        })
    }

    for field in view.scalar_fields() {
        dbg!((field.name(), field.arity()));
        rendered.push_field(scalar_field::render(field, warnings));
    }

    // TODO: relation fields

    rendered
}

fn render_id<'a>(id: walkers::PrimaryKeyWalker<'a>) -> renderer::IdDefinition<'a> {
    let fields = id.scalar_field_attributes().map(|attrs| {
        let field = attrs.as_index_field();
        let mut rendered = renderer::IndexFieldInput::new(field.name());

        if let Some(SortOrder::Desc) = attrs.sort_order() {
            rendered.sort_order("Desc");
        }

        if let Some(length) = attrs.length() {
            rendered.length(length);
        }

        rendered
    });

    let mut definition = renderer::IdDefinition::new(fields);

    if let Some(name) = id.name() {
        definition.name(name);
    }

    if let Some(map) = id.mapped_name() {
        definition.map(map);
    }

    if let Some(clustered) = id.clustered() {
        definition.clustered(clustered);
    }

    definition
}

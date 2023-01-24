use crate::{
    datamodel_calculator::{InputContext, OutputContext},
    introspection_helpers as helpers,
    pair::ViewPair,
    warnings::{self, Warnings},
};
use datamodel_renderer::datamodel as renderer;

use super::{id, indexes, relation_field, scalar_field};

/// Render all view blocks to the PSL.
pub(super) fn render<'a>(input: InputContext<'a>, output: &mut OutputContext<'a>) {
    let mut views_with_idx: Vec<(Option<_>, renderer::View<'a>)> = Vec::with_capacity(input.schema.views_count());

    for view in input.view_pairs() {
        views_with_idx.push((view.previous_position(), render_view(view, &mut output.warnings)));
    }

    views_with_idx.sort_by(|(a, _), (b, _)| helpers::compare_options_none_last(*a, *b));

    for (_, render) in views_with_idx.into_iter() {
        output.rendered_schema.push_view(render);
    }
}

/// Render a single view.
fn render_view<'a>(view: ViewPair<'a>, warnings: &mut Warnings) -> renderer::View<'a> {
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
        rendered.id(id::render(id, warnings));
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

    if view.remapped_name() {
        warnings.remapped_views.push(warnings::View {
            view: view.name().to_string(),
        });
    }

    for field in view.scalar_fields() {
        rendered.push_field(scalar_field::render(field, warnings));
    }

    for field in view.relation_fields() {
        rendered.push_field(relation_field::render(field, warnings));
    }

    for definition in view.indexes().map(indexes::render) {
        rendered.push_index(definition);
    }

    rendered
}

use super::{id, indexes, relation_field, scalar_field};
use crate::introspection::{
    datamodel_calculator::DatamodelCalculatorContext, introspection_helpers as helpers, introspection_pair::ViewPair,
};
use datamodel_renderer::datamodel as renderer;
use schema_connector::ViewDefinition;
use std::borrow::Cow;

/// Render all view blocks to the PSL.
pub(super) fn render<'a>(
    introspection_file_name: &'a str,
    ctx: &'a DatamodelCalculatorContext<'a>,
    rendered: &mut renderer::Datamodel<'a>,
) -> Vec<ViewDefinition> {
    let mut definitions = Vec::new();
    let mut views_with_idx: Vec<(Option<_>, renderer::View<'a>)> = Vec::with_capacity(ctx.sql_schema.views_count());

    for view in ctx.view_pairs() {
        if let Some(definition) = view.definition() {
            let schema = view
                .namespace()
                .map(ToString::to_string)
                .unwrap_or_else(|| ctx.search_path.to_string());

            definitions.push(ViewDefinition {
                schema,
                name: view.name().to_string(),
                definition: definition.to_string(),
            });
        }

        views_with_idx.push((view.previous_position(), render_view(view)));
    }

    views_with_idx.sort_by(|(a, _), (b, _)| helpers::compare_options_none_last(*a, *b));

    for (previous_view, render) in views_with_idx.into_iter() {
        let file_name = match previous_view {
            Some((previous_file_id, _)) => ctx.previous_schema.db.file_name(previous_file_id),
            None => introspection_file_name,
        };

        rendered.push_view(Cow::Borrowed(file_name), render);
    }

    definitions
}

/// Render a single view.
fn render_view(view: ViewPair<'_>) -> renderer::View<'_> {
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
        rendered.id(id::render(id));
    }

    if !view.has_usable_identifier() && !view.ignored_in_psl() {
        let docs = "The underlying view does not contain a valid unique identifier and can therefore currently not be handled by Prisma Client.";
        rendered.documentation(docs);
    }

    if view.adds_a_description() {
        let docs = "This view or at least one of its fields has comments in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments";
        rendered.documentation(docs);
    }

    for field in view.scalar_fields() {
        rendered.push_field(scalar_field::render(field));
    }

    for field in view.relation_fields() {
        rendered.push_field(relation_field::render(field));
    }

    for definition in view.indexes().map(indexes::render) {
        rendered.push_index(definition);
    }

    rendered
}

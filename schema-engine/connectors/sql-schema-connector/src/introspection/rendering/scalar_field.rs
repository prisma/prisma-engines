//! Rendering of model scalar fields.

use crate::introspection::{
    introspection_pair::{IdPair, IndexPair, ScalarFieldPair},
    rendering::defaults,
};
use datamodel_renderer::datamodel as renderer;
use sql_schema_describer::ColumnArity;

/// Render a scalar field to be added in a model.
pub(crate) fn render(field: ScalarFieldPair<'_>) -> renderer::Field<'_> {
    let mut rendered = renderer::Field::new(field.name(), field.prisma_type());

    match field.arity() {
        ColumnArity::Nullable => rendered.optional(),
        ColumnArity::List => rendered.array(),
        ColumnArity::Required => (),
    }

    if field.is_unsupported() {
        rendered.unsupported();
    }

    if let Some(map) = field.mapped_name() {
        rendered.map(map);
    }

    if let Some((prefix, r#type, params)) = field.native_type() {
        rendered.native_type(prefix, r#type, params)
    }

    if let Some(docs) = field.documentation() {
        rendered.documentation(docs);
    }

    if let Some(default) = defaults::render(&field) {
        rendered.default(default);
    }

    if field.is_updated_at() {
        rendered.updated_at();
    }

    if field.is_ignored() {
        rendered.ignore();
    }

    if let Some(pk) = field.id() {
        rendered.id(render_id(pk));
    }

    if let Some(unique) = field.unique() {
        rendered.unique(render_unique(unique));
    }

    if field.remapped_name_empty() {
        let docs = "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*";

        rendered.documentation(docs);
        rendered.commented_out();
    }

    rendered
}

/// Render a `@id` definition to a field.
fn render_id(pk: IdPair<'_>) -> renderer::IdFieldDefinition<'_> {
    let field = pk.field().unwrap();
    let mut definition = renderer::IdFieldDefinition::default();

    if let Some(clustered) = pk.clustered() {
        definition.clustered(clustered);
    }

    if let Some(sort_order) = field.sort_order() {
        definition.sort_order(sort_order);
    }

    if let Some(length) = field.length() {
        definition.length(length);
    }

    if let Some(map) = pk.mapped_name() {
        definition.map(map);
    }

    definition
}

/// Render a `@unique` definition to a field.
fn render_unique(unique: IndexPair<'_>) -> renderer::UniqueFieldAttribute<'_> {
    let mut opts = renderer::UniqueFieldAttribute::default();
    let field = unique.field().unwrap();

    if let Some(map) = unique.mapped_name() {
        opts.map(map);
    }

    if let Some(clustered) = unique.clustered() {
        opts.clustered(clustered);
    }

    if let Some(sort_order) = field.sort_order() {
        opts.sort_order(sort_order);
    }

    if let Some(length) = field.length() {
        opts.length(length);
    }

    opts
}

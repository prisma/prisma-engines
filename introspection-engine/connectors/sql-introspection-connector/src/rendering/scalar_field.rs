//! Rendering of model scalar fields.

use crate::{
    pair::{IdPair, IndexPair, ScalarFieldPair},
    rendering::defaults,
    warnings::{self, Warnings},
};
use datamodel_renderer::datamodel as renderer;
use either::Either;
use sql_schema_describer::ColumnArity;

/// Render a scalar field to be added in a model.
pub(crate) fn render<'a>(field: ScalarFieldPair<'a>, warnings: &mut Warnings) -> renderer::Field<'a> {
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

    if let Some(default) = defaults::render(field, warnings) {
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

    if field.remapped_name_from_psl() {
        match field.container() {
            Either::Left(model) => {
                let mf = crate::warnings::ModelAndField {
                    model: model.name().to_string(),
                    field: field.name().to_string(),
                };

                warnings.remapped_fields_in_model.push(mf);
            }
            Either::Right(view) => {
                let mf = crate::warnings::ViewAndField {
                    view: view.name().to_string(),
                    field: field.name().to_string(),
                };

                warnings.remapped_fields_in_view.push(mf);
            }
        }
    }

    if field.is_unsupported() {
        match field.container() {
            Either::Left(model) => {
                let mf = warnings::ModelAndFieldAndType {
                    model: model.name().to_string(),
                    field: field.name().to_string(),
                    tpe: field.prisma_type().to_string(),
                };

                warnings.unsupported_types_in_model.push(mf)
            }
            Either::Right(view) => {
                let mf = warnings::ViewAndFieldAndType {
                    view: view.name().to_string(),
                    field: field.name().to_string(),
                    tpe: field.prisma_type().to_string(),
                };

                warnings.unsupported_types_in_view.push(mf)
            }
        }
    }

    if field.remapped_name_empty() {
        let docs = "This field was commented out because of an invalid name. Please provide a valid one that matches [a-zA-Z][a-zA-Z0-9_]*";
        rendered.documentation(docs);
        rendered.commented_out();

        match field.container() {
            Either::Left(model) => {
                let mf = crate::warnings::ModelAndField {
                    model: model.name().to_string(),
                    field: field.name().to_string(),
                };

                warnings.fields_with_empty_names_in_model.push(mf);
            }
            Either::Right(view) => {
                let mf = crate::warnings::ViewAndField {
                    view: view.name().to_string(),
                    field: field.name().to_string(),
                };

                warnings.fields_with_empty_names_in_view.push(mf);
            }
        }
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

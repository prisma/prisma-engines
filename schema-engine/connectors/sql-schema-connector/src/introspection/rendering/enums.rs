//! Rendering of enumerators.

use std::borrow::Cow;

use crate::introspection::{
    datamodel_calculator::DatamodelCalculatorContext, introspection_helpers as helpers, introspection_pair::EnumPair,
    sanitize_datamodel_names,
};
use datamodel_renderer::datamodel as renderer;
use psl::parser_database as db;

/// Render all enums.
pub(super) fn render<'a>(
    introspection_file_name: &'a str,
    ctx: &'a DatamodelCalculatorContext<'a>,
    rendered: &mut renderer::Datamodel<'a>,
) {
    let mut all_enums: Vec<(Option<db::EnumId>, renderer::Enum<'_>)> = Vec::new();

    for pair in ctx.enum_pairs() {
        all_enums.push((pair.previous_position(), render_enum(pair)))
    }

    all_enums.sort_by(|(id_a, _), (id_b, _)| helpers::compare_options_none_last(id_a.as_ref(), id_b.as_ref()));

    if ctx.sql_family.is_mysql() {
        // MySQL can have multiple database enums matching one Prisma enum.
        all_enums.dedup_by(|(id_a, _), (id_b, _)| match (id_a, id_b) {
            (Some(id_a), Some(id_b)) => id_a == id_b,
            _ => false,
        });
    }

    for (previous_schema_enum, enm) in all_enums {
        let file_name = match previous_schema_enum {
            Some((prev_file_id, _)) => ctx.previous_schema.db.file_name(prev_file_id),
            None => introspection_file_name,
        };

        rendered.push_enum(Cow::Borrowed(file_name), enm);
    }
}

/// Render a single enum.
fn render_enum(r#enum: EnumPair<'_>) -> renderer::Enum<'_> {
    let mut rendered_enum = renderer::Enum::new(r#enum.name());

    if let Some(schema) = r#enum.namespace() {
        rendered_enum.schema(schema);
    }

    if let Some(mapped_name) = r#enum.mapped_name() {
        rendered_enum.map(mapped_name);
    }

    if let Some(docs) = r#enum.documentation() {
        rendered_enum.documentation(docs);
    }

    if r#enum.adds_a_description() {
        let docs = "This enum is commented in the database, and requires an additional setup for migrations: Read more: https://pris.ly/d/database-comments";
        rendered_enum.documentation(docs);
    }

    for variant in r#enum.variants() {
        let mut rendered_variant = renderer::EnumVariant::new(variant.name());

        if let Some(docs) = variant.documentation() {
            rendered_variant.documentation(docs);
        }

        if let Some(map) = variant.mapped_name() {
            rendered_variant.map(map);
        }

        if variant.name().is_empty() || sanitize_datamodel_names::needs_sanitation(&variant.name()) {
            rendered_variant.comment_out();
        }

        rendered_enum.push_variant(rendered_variant);
    }

    rendered_enum
}

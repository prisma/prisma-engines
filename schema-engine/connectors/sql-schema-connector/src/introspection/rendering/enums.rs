//! Rendering of enumerators.

use crate::introspection::{
    datamodel_calculator::DatamodelCalculatorContext, introspection_helpers as helpers, introspection_pair::EnumPair,
    sanitize_datamodel_names,
};
use datamodel_renderer::datamodel as renderer;
use psl::{diagnostics::FileId, parser_database as db, schema_ast::ast::EnumId};

/// Render all enums.
pub(super) fn render<'a>(
    introspection_file_name: &str,
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

        rendered.push_enum(file_name.to_string(), enm);
    }

    let removed_enums = compute_removed_enums(ctx);

    // Ensures that if an enum is removed, the file remains present in the result.
    for (prev_file_id, _) in removed_enums {
        let file_name = ctx.previous_schema.db.file_name(prev_file_id);

        rendered.create_empty_file(file_name.to_owned());
    }
}

fn compute_removed_enums(ctx: &DatamodelCalculatorContext<'_>) -> Vec<(FileId, EnumId)> {
    let mut removed_enums = Vec::new();

    for eenums in ctx.previous_schema.db.walk_enums() {
        let previous_enum = (eenums.id.0, eenums.id.1);

        if !ctx
            .enum_pairs()
            .any(|eenum| eenum.previous_position() == Some(previous_enum))
        {
            removed_enums.push(previous_enum);
        }
    }

    removed_enums
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

//! Rendering of enumerators.

use crate::{
    datamodel_calculator::{InputContext, OutputContext},
    introspection_helpers as helpers,
    pair::EnumPair,
    sanitize_datamodel_names, warnings,
};
use datamodel_renderer::datamodel as renderer;
use psl::parser_database::ast;

/// Render all enums.
pub(super) fn render<'a>(input: InputContext<'a>, output: &mut OutputContext<'a>) {
    let mut all_enums: Vec<(Option<ast::EnumId>, renderer::Enum)> = Vec::new();

    for pair in input.enum_pairs() {
        let rendered_enum = render_enum(pair, output);
        all_enums.push((pair.previous_position(), rendered_enum))
    }

    all_enums.sort_by(|(id_a, _), (id_b, _)| helpers::compare_options_none_last(id_a.as_ref(), id_b.as_ref()));

    if input.sql_family.is_mysql() {
        // MySQL can have multiple database enums matching one Prisma enum.
        all_enums.dedup_by(|(id_a, _), (id_b, _)| match (id_a, id_b) {
            (Some(id_a), Some(id_b)) => id_a == id_b,
            _ => false,
        });
    }

    for (_, enm) in all_enums {
        output.rendered_schema.push_enum(enm);
    }
}

/// Render a single enum.
fn render_enum<'a>(r#enum: EnumPair<'a>, output: &mut OutputContext<'a>) -> renderer::Enum<'a> {
    let mut remapped_values = Vec::new();
    let mut rendered_enum = renderer::Enum::new(r#enum.name());

    if let Some(schema) = r#enum.namespace() {
        rendered_enum.schema(schema);
    }

    if let Some(mapped_name) = r#enum.mapped_name() {
        rendered_enum.map(mapped_name);

        let warning = warnings::warning_enriched_with_map_on_enum(&[warnings::Enum::new(&r#enum.name())]);
        output.warnings.push(warning);
    }

    if let Some(docs) = r#enum.documentation() {
        rendered_enum.documentation(docs);
    }

    for variant in r#enum.variants() {
        if variant.name().is_empty() {
            let value = variant
                .mapped_name()
                .map(String::from)
                .unwrap_or_else(|| variant.name().to_string());

            let warning = warnings::EnumAndValue {
                enm: r#enum.name().to_string(),
                value,
            };

            output.warnings.enum_values_with_empty_names.push(warning);
        }

        let mut rendered_variant = renderer::EnumVariant::new(variant.name());

        if let Some(docs) = variant.documentation() {
            rendered_variant.documentation(docs);
        }

        if let Some(map) = variant.mapped_name() {
            rendered_variant.map(map);
        }

        if variant.name().is_empty() || sanitize_datamodel_names::needs_sanitation(&variant.name()) {
            let warning = warnings::EnumAndValue {
                enm: r#enum.name().to_string(),
                value: variant.name().to_string(),
            };

            output.warnings.enum_values_with_empty_names.push(warning);
            rendered_variant.comment_out();
        } else if variant.mapped_name().is_some() {
            remapped_values.push(warnings::EnumAndValue {
                value: variant.name().to_string(),
                enm: r#enum.name().to_string(),
            });
        }

        rendered_enum.push_variant(rendered_variant);
    }

    if !remapped_values.is_empty() {
        output
            .warnings
            .push(warnings::warning_enriched_with_map_on_enum_value(&remapped_values))
    }

    rendered_enum
}

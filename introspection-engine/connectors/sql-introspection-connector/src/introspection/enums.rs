use std::borrow::Cow;

use crate::{introspection::Context, introspection_helpers::*, warnings, EnumVariantName, ModelName};
use datamodel_renderer::datamodel as renderer;
use psl::{
    parser_database::{ast, walkers},
    schema_ast::ast::WithDocumentation,
};
use sql_schema_describer as sql;

pub(super) fn introspect_enums(ctx: &mut Context<'_>) {
    let mut all_enums: Vec<(Option<ast::EnumId>, renderer::Enum)> = ctx
        .schema
        .enum_walkers()
        .map(|enm| {
            let existing_enum = ctx.existing_enum(enm.id);
            let rendered_enum = render_enum(enm, existing_enum, ctx);
            (existing_enum.map(|e| e.id), rendered_enum)
        })
        .collect();

    all_enums.sort_by(|(id_a, _), (id_b, _)| compare_options_none_last(id_a.as_ref(), id_b.as_ref()));

    if ctx.sql_family.is_mysql() {
        // MySQL can have multiple database enums matching one Prisma enum.
        all_enums.dedup_by(|(id_a, _), (id_b, _)| match (id_a, id_b) {
            (Some(id_a), Some(id_b)) => id_a == id_b,
            _ => false,
        });
    }

    for (_, enm) in all_enums {
        ctx.rendered_schema.push_enum(enm);
    }
}

fn render_enum<'a>(
    sql_enum: sql::EnumWalker<'a>,
    existing_enum: Option<walkers::EnumWalker<'a>>,
    ctx: &mut Context<'a>,
) -> renderer::Enum<'a> {
    let mut remapped_values = Vec::new();
    let schema = if matches!(ctx.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
        sql_enum.namespace()
    } else {
        None
    };
    let (enum_name, enum_database_name) = match ctx.enum_prisma_name(sql_enum.id) {
        ModelName::FromPsl { name, mapped_name } => (Cow::Borrowed(name), mapped_name),
        ModelName::FromSql { name } => (Cow::Borrowed(name), None),
        name @ ModelName::RenamedReserved { mapped_name } | name @ ModelName::RenamedSanitized { mapped_name } => {
            (name.prisma_name(), Some(mapped_name))
        }
    };
    let mut rendered_enum = renderer::Enum::new(enum_name.clone());

    if let Some(schema) = schema {
        rendered_enum.schema(schema);
    }

    if let Some(mapped_name) = enum_database_name {
        rendered_enum.map(mapped_name);
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum(&[warnings::Enum::new(
                &enum_name,
            )]));
    }

    if let Some(docs) = existing_enum.and_then(|e| e.ast_enum().documentation()) {
        rendered_enum.documentation(docs);
    }

    for sql_variant in sql_enum.variants() {
        let variant_name = ctx.enum_variant_name(sql_variant.id);
        let mut prisma_name = variant_name.prisma_name();
        let mapped_name = variant_name.mapped_name();
        let mut comment_out = false;

        match variant_name {
            EnumVariantName::RenamedSanitized { mapped_name } if prisma_name.is_empty() => {
                ctx.enum_values_with_empty_names.push(warnings::EnumAndValue {
                    enm: enum_name.clone().into_owned(),
                    value: mapped_name.to_owned(),
                });
                prisma_name = Cow::Borrowed(mapped_name);
                comment_out = true;
            }
            EnumVariantName::FromPsl {
                mapped_name: Some(_), ..
            } => {
                remapped_values.push(warnings::EnumAndValue {
                    value: prisma_name.clone().into_owned(),
                    enm: enum_name.clone().into_owned(),
                });
            }
            _ => (),
        }

        let existing_value = existing_enum.and_then(|enm| {
            enm.values()
                .find(|val| val.database_name() == mapped_name.as_deref().unwrap_or(prisma_name.as_ref()))
        });

        let mut rendered_variant = renderer::EnumVariant::new(prisma_name);
        rendered_variant.documentation(existing_value.and_then(|v| v.documentation()));
        rendered_variant.map(mapped_name);
        rendered_variant.comment_out(comment_out);

        rendered_enum.push_variant(rendered_variant);
    }

    if !remapped_values.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum_value(&remapped_values))
    }

    rendered_enum
}

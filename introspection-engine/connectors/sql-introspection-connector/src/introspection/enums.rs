use crate::{introspection::Context, introspection_helpers::*, warnings, EnumVariantName, ModelName};
use psl::{
    dml,
    parser_database::{ast, walkers},
    schema_ast::ast::WithDocumentation,
};
use sql_schema_describer as sql;

pub(super) fn introspect_enums(datamodel: &mut dml::Datamodel, ctx: &mut Context<'_>) {
    let mut all_enums: Vec<(Option<ast::EnumId>, dml::Enum)> = ctx
        .schema
        .enum_walkers()
        .map(|enm| {
            let existing_enum = ctx.existing_enum(enm.id);
            let dml_enum = sql_enum_to_dml_enum(enm, existing_enum, ctx);
            (existing_enum.map(|e| e.id), dml_enum)
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

    datamodel.enums = all_enums.into_iter().map(|(_id, dml_enum)| dml_enum).collect();
}

fn sql_enum_to_dml_enum(
    sql_enum: sql::EnumWalker<'_>,
    existing_enum: Option<walkers::EnumWalker<'_>>,
    ctx: &mut Context,
) -> dml::Enum {
    let schema = if matches!(ctx.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
        sql_enum.namespace().map(String::from)
    } else {
        None
    };
    let (enum_name, enum_database_name) = match ctx.enum_prisma_name(sql_enum.id) {
        ModelName::FromPsl { name, mapped_name } => (name.to_owned(), mapped_name),
        ModelName::FromSql { name } => (name.to_owned(), None),
        name @ ModelName::RenamedReserved { mapped_name } | name @ ModelName::RenamedSanitized { mapped_name } => {
            (name.prisma_name().into_owned(), Some(mapped_name))
        }
    };
    let mut dml_enum = dml::Enum::new(&enum_name, Vec::new(), schema);
    dml_enum.database_name = enum_database_name.map(ToOwned::to_owned);
    dml_enum.documentation = existing_enum
        .and_then(|enm| enm.ast_enum().documentation())
        .map(ToOwned::to_owned);

    if dml_enum.database_name.is_some() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum(&[warnings::Enum::new(
                &enum_name,
            )]));
    }

    dml_enum.values.reserve(sql_enum.values().len());
    let mut remapped_values = Vec::new(); // for warnings

    for sql_variant in sql_enum.variants() {
        let mut dml_value = dml::EnumValue::new("");
        let variant_name = ctx.enum_variant_name(sql_variant.id);
        let mut prisma_name = variant_name.prisma_name().into_owned();
        let mapped_name = variant_name.mapped_name().map(ToOwned::to_owned);

        match variant_name {
            EnumVariantName::RenamedSanitized { mapped_name } if prisma_name.is_empty() => {
                prisma_name = mapped_name.to_owned();
                dml_value.commented_out = true;
            }
            EnumVariantName::FromPsl {
                mapped_name: Some(_), ..
            } => {
                remapped_values.push(warnings::EnumAndValue {
                    value: prisma_name.clone(),
                    enm: enum_name.to_owned(),
                });
            }
            _ => (),
        }

        let existing_value = existing_enum.and_then(|enm| {
            enm.values()
                .find(|val| val.database_name() == mapped_name.as_ref().unwrap_or(&prisma_name))
        });
        dml_value.documentation = existing_value.and_then(|v| v.documentation()).map(ToOwned::to_owned);
        dml_value.name = prisma_name;
        dml_value.database_name = mapped_name;

        dml_enum.values.push(dml_value);
    }

    if !remapped_values.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum_value(&remapped_values))
    }

    dml_enum
}

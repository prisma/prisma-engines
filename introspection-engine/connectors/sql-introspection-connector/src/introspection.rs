pub(crate) mod inline_relations;

mod m2m_relations;
mod postgres;
mod prisma_relation_mode;

use crate::{
    calculate_datamodel::CalculateDatamodelContext as Context,
    commenting_out_guardrails::commenting_out_guardrails,
    introspection_helpers::*,
    prisma_1_defaults::add_prisma_1_id_defaults,
    re_introspection::enrich,
    sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names},
    version_checker, warnings, SqlError, SqlFamilyTrait,
};
use datamodel_renderer as render;
use introspection_connector::Version;
use psl::{
    datamodel_connector::constraint_names::ConstraintNames,
    dml::{self, Datamodel, Field, Model, PrimaryKeyDefinition, PrimaryKeyField, SortOrder},
    parser_database::{ast, walkers},
    schema_ast::ast::WithDocumentation,
    Configuration,
};
use sql_schema_describer::{self as sql, SQLSortOrder, SqlSchema};
use std::collections::HashMap;

pub(crate) fn introspect(ctx: &mut Context) -> Result<(Version, String, bool), SqlError> {
    let mut datamodel = Datamodel::new();

    introspect_enums(&mut datamodel, ctx);
    introspect_models(&mut datamodel, ctx);

    if ctx.foreign_keys_enabled() {
        inline_relations::introspect_inline_relations(&mut datamodel, ctx);
    } else {
        prisma_relation_mode::reintrospect_relations(&mut datamodel, ctx);
    }

    if !sanitization_leads_to_duplicate_names(&datamodel) {
        // our opinionation about valid names
        sanitize_datamodel_names(ctx, &mut datamodel);
    }

    // TODO(matthias) relation field names might be different since they do not come from the schema but we generate them during dml construction
    // deduplicating relation field names
    deduplicate_relation_field_names(&mut datamodel);

    if !ctx.previous_datamodel.is_empty() {
        enrich(ctx.previous_datamodel, &mut datamodel, ctx);
    }

    // commenting out models, fields, enums, enum values
    ctx.warnings.append(&mut commenting_out_guardrails(&mut datamodel, ctx));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_checker::check_prisma_version(ctx);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&version, &mut datamodel, ctx.schema, ctx);

    let config = if ctx.render_config {
        render_configuration(ctx.config, ctx.schema).to_string()
    } else {
        String::new()
    };

    m2m_relations::introspect_m2m_relations(&mut datamodel, ctx);

    // Ordering of model fields.
    //
    // This sorts backrelation field after relation fields, in order to preserve an ordering
    // similar to that of the previous implementation.
    for model in &mut datamodel.models {
        model
            .fields
            .sort_by(|a, b| match (a.as_relation_field(), b.as_relation_field()) {
                (Some(a), Some(b)) if a.relation_info.fields.is_empty() && !b.relation_info.fields.is_empty() => {
                    std::cmp::Ordering::Greater // back relation fields last
                }
                (Some(a), Some(b)) if b.relation_info.fields.is_empty() && !a.relation_info.fields.is_empty() => {
                    std::cmp::Ordering::Less
                }
                _ => std::cmp::Ordering::Equal,
            });
    }

    let rendered = format!(
        "{}\n{}",
        config,
        render::Datamodel::from_dml(&ctx.config.datasources[0], &datamodel),
    );

    Ok((version, psl::reformat(&rendered, 2).unwrap(), datamodel.is_empty()))
}

fn render_configuration<'a>(config: &'a Configuration, schema: &'a SqlSchema) -> render::Configuration<'a> {
    let mut output = render::Configuration::default();
    let prev_ds = config.datasources.first().unwrap();
    let mut datasource = render::configuration::Datasource::from_psl(prev_ds);

    if prev_ds.active_connector.is_provider("postgres") {
        postgres::add_extensions(&mut datasource, schema, config);
    }

    output.push_datasource(datasource);

    for prev in config.generators.iter() {
        output.push_generator(render::configuration::Generator::from_psl(prev));
    }

    output
}

fn introspect_enums(datamodel: &mut Datamodel, ctx: &mut Context<'_>) {
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

    if ctx.sql_family().is_mysql() {
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
    let enum_name = existing_enum.map(|enm| enm.name()).unwrap_or_else(|| sql_enum.name());
    let mut dml_enum = dml::Enum::new(enum_name, Vec::new(), schema);

    dml_enum.documentation = existing_enum
        .and_then(|enm| enm.ast_enum().documentation())
        .map(ToOwned::to_owned);

    dml_enum.database_name = if ctx.sql_family.is_mysql() {
        existing_enum.and_then(|enm| enm.mapped_name()).map(ToOwned::to_owned)
    } else {
        existing_enum
            .filter(|existing_enum| existing_enum.name() != sql_enum.name())
            .map(|_| sql_enum.name().to_owned())
    };

    if dml_enum.database_name.is_some() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum(&[warnings::Enum::new(
                enum_name,
            )]));
    }

    dml_enum.values.reserve(sql_enum.values().len());

    let mut remapped_values = Vec::new(); // for warnings

    for value in sql_enum.values() {
        let mut dml_value = dml::EnumValue::new(value);
        let existing_value = existing_enum.and_then(|enm| enm.values().find(|val| val.database_name() == value));
        dml_value.documentation = existing_value.and_then(|v| v.documentation()).map(ToOwned::to_owned);

        // Re-introspect mapped names.
        if let Some(existing_value) = existing_value.filter(|val| val.mapped_name().is_some()) {
            let mapped_name = std::mem::replace(&mut dml_value.name, existing_value.name().to_owned());
            dml_value.database_name = Some(mapped_name);
            remapped_values.push(warnings::EnumAndValue {
                value: dml_value.name.clone(),
                enm: enum_name.to_owned(),
            });
        }

        dml_enum.values.push(dml_value);
    }

    if !remapped_values.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_enum_value(&remapped_values))
    }

    dml_enum
}

fn introspect_models(datamodel: &mut Datamodel, ctx: &mut Context<'_>) {
    let mut re_introspected_model_ignores = Vec::new();
    let mut remapped_models = Vec::new();
    let mut remapped_fields = Vec::new();
    let mut reintrospected_id_names = Vec::new();

    for table in ctx
        .schema
        .table_walkers()
        .filter(|table| !is_old_migration_table(*table))
        .filter(|table| !is_new_migration_table(*table))
        .filter(|table| !is_prisma_join_table(*table))
        .filter(|table| !is_relay_table(*table))
    {
        let existing_model = ctx.existing_model(table.id);
        let model_name = existing_model.map(|m| m.name()).unwrap_or_else(|| table.name());
        let mut model = Model::new(model_name.to_owned(), None);

        if let Some(m) = existing_model.filter(|m| m.mapped_name().is_some()) {
            remapped_models.push(warnings::Model {
                model: m.name().to_owned(),
            });
        }

        for column in table.columns() {
            model.add_field(Field::ScalarField(calculate_scalar_field(
                column,
                &mut remapped_fields,
                ctx,
            )));
        }

        for index in table.indexes() {
            if let Some(index) = calculate_index(index, ctx) {
                model.add_index(index);
            }
        }

        if let Some(pk) = table.primary_key() {
            let clustered = primary_key_is_clustered(pk.id, ctx);
            let name = existing_model
                .and_then(|model| model.primary_key())
                .and_then(|pk| pk.name())
                .map(ToOwned::to_owned);

            if name.is_some() {
                reintrospected_id_names.push(warnings::Model {
                    model: existing_model.unwrap().name().to_owned(),
                });
            }

            let db_name = if pk.name() == ConstraintNames::primary_key_name(table.name(), ctx.active_connector())
                || pk.name().is_empty()
            {
                None
            } else {
                Some(pk.name().to_owned())
            };

            model.primary_key = Some(PrimaryKeyDefinition {
                name,
                db_name,
                fields: pk
                    .columns()
                    .map(|c| {
                        let sort_order = c.sort_order().and_then(|sort| match sort {
                            SQLSortOrder::Asc => None,
                            SQLSortOrder::Desc => Some(SortOrder::Desc),
                        });

                        PrimaryKeyField {
                            name: ctx.column_prisma_name(c.as_column().id).to_owned(),
                            sort_order,
                            length: c.length(),
                        }
                    })
                    .collect(),
                defined_on_field: pk.columns().len() == 1,
                clustered,
            });
        }

        if matches!(ctx.config.datasources.first(), Some(ds) if !ds.namespaces.is_empty()) {
            model.schema = table.namespace().map(|n| n.to_string());
        }

        model.database_name = existing_model
            .filter(|model| model.name() != table.name())
            .map(|_| table.name().to_owned());

        model.documentation = existing_model
            .and_then(|model| model.ast_model().documentation())
            .map(ToOwned::to_owned);

        if existing_model.map(|model| model.is_ignored()).unwrap_or(false) {
            model.is_ignored = true;
            re_introspected_model_ignores.push(warnings::Model {
                model: model_name.to_owned(),
            });
        }

        datamodel.models.push(model);
    }

    if !remapped_models.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_model(&remapped_models));
    }

    if !remapped_fields.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_map_on_field(&remapped_fields));
    }

    if !reintrospected_id_names.is_empty() {
        ctx.warnings
            .push(warnings::warning_enriched_with_custom_primary_key_names(
                &reintrospected_id_names,
            ))
    }

    sort_models(datamodel, ctx)
}

fn sort_models(datamodel: &mut Datamodel, ctx: &Context) {
    let existing_models_by_database_name: HashMap<&str, _> = ctx
        .previous_schema
        .db
        .walk_models()
        .map(|model| (model.database_name(), model.id))
        .collect();

    datamodel.models.sort_by(|a, b| {
        let existing = |model: &dml::Model| -> Option<_> {
            existing_models_by_database_name.get(model.database_name.as_deref().unwrap_or(&model.name))
        };

        compare_options_none_last(existing(a), existing(b))
    });
}

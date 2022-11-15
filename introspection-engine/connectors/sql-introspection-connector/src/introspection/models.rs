use crate::{introspection::Context, introspection_helpers::*, warnings};
use psl::{datamodel_connector::constraint_names::ConstraintNames, dml, schema_ast::ast::WithDocumentation};
use sql_schema_describer as sql;
use std::collections::HashMap;

pub(super) fn introspect_models(datamodel: &mut dml::Datamodel, ctx: &mut Context<'_>) {
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
        let mut model = dml::Model::new(String::new(), None);
        let mut documentation = String::new();

        match ctx.table_prisma_name(table.id) {
            crate::ModelName::FromPsl { name, mapped_name } => {
                model.name = name.to_owned();
                model.database_name = mapped_name.map(ToOwned::to_owned);
            }
            crate::ModelName::FromSql { name } => {
                model.name = name.to_owned();
            }
            model_name @ crate::ModelName::RenamedReserved { mapped_name } => {
                let renamed = model_name.prisma_name().into_owned();
                documentation.push_str(&format!(
                    "This model has been renamed to '{renamed}' during introspection, because the original name '{mapped_name}' is reserved.",
                ));
                model.name = renamed;
                model.database_name = Some(table.name().to_owned());
            }
            model_name @ crate::ModelName::RenamedSanitized { mapped_name: _ } => {
                model.name = model_name.prisma_name().into_owned();
                model.database_name = Some(table.name().to_owned());
            }
        }

        if let Some(m) = existing_model.filter(|m| m.mapped_name().is_some()) {
            remapped_models.push(warnings::Model {
                model: m.name().to_owned(),
            });
        }

        if let Some(docs) = existing_model.and_then(|m| m.ast_model().documentation()) {
            documentation.push_str(docs);
        }

        for column in table.columns() {
            model.add_field(dml::Field::ScalarField(calculate_scalar_field(
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

            model.primary_key = Some(dml::PrimaryKeyDefinition {
                name,
                db_name,
                fields: pk
                    .columns()
                    .map(|c| {
                        let sort_order = c.sort_order().and_then(|sort| match sort {
                            sql::SQLSortOrder::Asc => None,
                            sql::SQLSortOrder::Desc => Some(dml::SortOrder::Desc),
                        });

                        dml::PrimaryKeyField {
                            name: ctx.column_prisma_name(c.as_column().id).prisma_name().into_owned(),
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

        model.documentation = Some(documentation).filter(|doc| !doc.is_empty());

        if existing_model.map(|model| model.is_ignored()).unwrap_or(false) {
            model.is_ignored = true;
            re_introspected_model_ignores.push(warnings::Model {
                model: model.name.clone(),
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

fn sort_models(datamodel: &mut dml::Datamodel, ctx: &Context) {
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

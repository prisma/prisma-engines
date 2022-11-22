use crate::{introspection::Context, introspection_helpers::*, warnings};
use psl::{datamodel_connector::constraint_names::ConstraintNames, dml, schema_ast::ast::WithDocumentation};
use sql_schema_describer as sql;

pub(super) fn introspect_models(datamodel: &mut dml::Datamodel, ctx: &mut Context<'_>) {
    let mut models_with_idx: Vec<(Option<_>, sql::TableId, dml::Model)> = Vec::with_capacity(ctx.schema.tables_count());

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

        if table.columns().len() == 0 {
            documentation.push_str(empty_table_comment(ctx));
            model.is_commented_out = true;
            ctx.models_without_columns.push(warnings::Model {
                model: model.name.clone(),
            });
        } else if !table_has_usable_identifier(table) {
            ctx.models_without_identifiers.push(warnings::Model {
                model: model.name.clone(),
            });
            documentation.push_str("The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.");
            model.is_ignored = true;
        }

        if let Some(m) = existing_model.filter(|m| m.mapped_name().is_some()) {
            ctx.remapped_models.push(warnings::Model {
                model: m.name().to_owned(),
            });
        }

        if let Some(docs) = existing_model.and_then(|m| m.ast_model().documentation()) {
            documentation.push_str(docs);
        }

        for column in table.columns() {
            if let sql::ColumnTypeFamily::Unsupported(tpe) = column.column_type_family() {
                ctx.unsupported_types.push(warnings::ModelAndFieldAndType {
                    model: model.name.clone(),
                    field: ctx.column_prisma_name(column.id).prisma_name().into_owned(),
                    tpe: tpe.to_owned(),
                })
            }

            model.add_field(dml::Field::ScalarField(calculate_scalar_field(column, ctx)));
        }

        super::indexes::calculate_model_indexes(table, existing_model, &mut model, ctx);

        if let Some(pk) = table.primary_key() {
            let clustered = primary_key_is_clustered(pk.id, ctx);
            let name = existing_model
                .and_then(|model| model.primary_key())
                .and_then(|pk| pk.name())
                .map(ToOwned::to_owned);

            if name.is_some() {
                ctx.reintrospected_id_names.push(warnings::Model {
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
        }

        models_with_idx.push((existing_model.map(|w| w.id), table.id, model));
    }

    models_with_idx.sort_by(|(a, _, _), (b, _, _)| compare_options_none_last(*a, *b));

    for (idx, (_, table_id, dml_model)) in models_with_idx.into_iter().enumerate() {
        datamodel.models.push(dml_model);
        ctx.target_models.insert(table_id, idx);
    }
}

fn empty_table_comment(ctx: &mut Context<'_>) -> &'static str {
    // On postgres this is allowed, on the other dbs, this could be a symptom of missing privileges.
    if ctx.sql_family.is_postgres() {
        "We could not retrieve columns for the underlying table. Either it has none or you are missing rights to see them. Please check your privileges."
    } else {
        "We could not retrieve columns for the underlying table. You probably have no rights to see them. Please check your privileges."
    }
}

pub(super) fn table_has_usable_identifier(table: sql::TableWalker<'_>) -> bool {
    table
        .indexes()
        .filter(|idx| idx.is_primary_key() || idx.is_unique())
        .any(|idx| {
            idx.columns().all(|c| {
                !matches!(
                    c.as_column().column_type().family,
                    sql::ColumnTypeFamily::Unsupported(_)
                ) && c.as_column().arity().is_required()
            })
        })
}

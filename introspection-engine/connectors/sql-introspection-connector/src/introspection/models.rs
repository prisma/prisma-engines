use std::borrow::Cow;

use crate::{introspection::Context, introspection_helpers::*, warnings};
use datamodel_renderer::datamodel as renderer;
use psl::{datamodel_connector::constraint_names::ConstraintNames, schema_ast::ast::WithDocumentation};
use sql_schema_describer as sql;

pub(super) fn render<'a>(ctx: &mut Context<'a>) {
    let mut models_with_idx: Vec<(Option<_>, sql::TableId, renderer::Model<'a>)> =
        Vec::with_capacity(ctx.schema.tables_count());

    for table in ctx
        .schema
        .table_walkers()
        .filter(|table| !is_old_migration_table(*table))
        .filter(|table| !is_new_migration_table(*table))
        .filter(|table| !is_prisma_join_table(*table))
        .filter(|table| !is_relay_table(*table))
    {
        let existing_model = ctx.existing_model(table.id);

        let (name, map, docs) = match ctx.table_prisma_name(table.id) {
            crate::ModelName::FromPsl { name, mapped_name } => (Cow::from(name), mapped_name, None),
            crate::ModelName::FromSql { name } => (Cow::from(name), None, None),
            model_name @ crate::ModelName::RenamedReserved { mapped_name } => {
                let docs =  format!(
                    "This model has been renamed to '{}' during introspection, because the original name '{}' is reserved.",
                    model_name.prisma_name(),
                    mapped_name,
                );

                (model_name.prisma_name(), Some(mapped_name), Some(docs))
            }
            model_name @ crate::ModelName::RenamedSanitized { mapped_name: _ } => {
                (model_name.prisma_name(), Some(table.name()), None)
            }
        };

        let mut model = renderer::Model::new(name.clone());

        if let Some(map) = map {
            model.map(map);
        }

        if let Some(docs) = docs {
            model.documentation(docs);
        }

        if table.columns().len() == 0 {
            model.documentation(empty_table_comment(ctx));
            model.comment_out();

            ctx.models_without_columns.push(warnings::Model {
                model: name.to_string(),
            });
        } else if !table_has_usable_identifier(table) {
            model.documentation("The underlying table does not contain a valid unique identifier and can therefore currently not be handled by the Prisma Client.");
            model.ignore();

            ctx.models_without_identifiers.push(warnings::Model {
                model: name.to_string(),
            });
        }

        if existing_model.filter(|m| m.mapped_name().is_some()).is_some() {
            ctx.remapped_models.push(warnings::Model {
                model: name.to_string(),
            });
        }

        if let Some(docs) = existing_model.and_then(|m| m.ast_model().documentation()) {
            model.documentation(docs);
        }

        for column in table.columns() {
            if let sql::ColumnTypeFamily::Unsupported(tpe) = column.column_type_family() {
                ctx.unsupported_types.push(warnings::ModelAndFieldAndType {
                    model: name.to_string(),
                    field: ctx.column_prisma_name(column.id).prisma_name().into_owned(),
                    tpe: tpe.to_owned(),
                })
            }

            let pk = table
                .primary_key()
                .filter(|pk| pk.columns().len() == 1)
                .filter(|pk| pk.contains_column(column.id));

            let unique = table
                .indexes()
                .filter(|i| i.is_unique())
                .filter(|i| i.columns().len() == 1)
                .find(|i| i.contains_column(column.id));

            model.push_field(render_scalar_field(column, pk, unique, ctx));
        }

        super::indexes::render_model_indexes(table, existing_model, &mut model, ctx);

        if let Some(pk) = table.primary_key() {
            let fields = pk.columns().map(|c| {
                let mut field = renderer::IndexFieldInput::new(ctx.column_prisma_name(c.as_column().id).prisma_name());

                if c.sort_order()
                    .filter(|o| matches!(o, sql::SQLSortOrder::Desc))
                    .is_some()
                {
                    field.sort_order("Desc");
                };

                if let Some(length) = c.length() {
                    field.length(length);
                }

                field
            });

            if fields.len() > 1 {
                let mut id = renderer::IdDefinition::new(fields);

                if let Some(name) = existing_model
                    .and_then(|model| model.primary_key())
                    .and_then(|pk| pk.name())
                {
                    id.name(name);

                    ctx.reintrospected_id_names.push(warnings::Model {
                        model: ctx.table_prisma_name(table.id).prisma_name().to_string(),
                    });
                }

                let default_name = ConstraintNames::primary_key_name(table.name(), ctx.active_connector());
                if pk.name() != default_name && !pk.name().is_empty() {
                    id.map(pk.name());
                }

                if let Some(clustered) = primary_key_is_clustered(pk.id, ctx) {
                    id.clustered(clustered);
                }

                model.id(id);
            }
        }

        match (table.namespace(), ctx.config.datasources.first()) {
            (Some(namespace), Some(ds)) if !ds.namespaces.is_empty() => {
                model.schema(namespace);
            }
            _ => (),
        }

        if existing_model.map(|model| model.is_ignored()).unwrap_or(false) {
            model.ignore();
        }

        models_with_idx.push((existing_model.map(|w| w.id), table.id, model));
    }

    models_with_idx.sort_by(|(a, _, _), (b, _, _)| compare_options_none_last(*a, *b));

    for (idx, (_, table_id, render)) in models_with_idx.into_iter().enumerate() {
        ctx.rendered_schema.push_model(render);
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

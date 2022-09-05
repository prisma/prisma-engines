use crate::{
    calculate_datamodel::CalculateDatamodelContext as Context,
    introspection_helpers::{
        calculate_backrelation_field, calculate_index, calculate_many_to_many_field, calculate_relation_field,
        calculate_scalar_field, is_new_migration_table, is_old_migration_table, is_prisma_1_point_0_join_table,
        is_prisma_1_point_1_or_2_join_table, is_relay_table, primary_key_is_clustered,
    },
    SqlError, SqlFamilyTrait,
};
use psl::dml::{self, Field, Model, PrimaryKeyDefinition, PrimaryKeyField, RelationField, SortOrder};
use sql_schema_describer::{walkers::TableWalker, ForeignKeyId, SQLSortOrder};
use std::collections::HashSet;
use tracing::debug;

pub(crate) fn introspect(ctx: &mut Context) -> Result<(), SqlError> {
    let schema = ctx.schema;
    // collect m2m table names
    let m2m_tables: Vec<String> = schema
        .table_walkers()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(*table) || is_prisma_1_point_0_join_table(*table))
        .map(|table| table.name()[1..].to_string())
        .collect();

    for table in schema
        .table_walkers()
        .filter(|table| !is_old_migration_table(*table))
        .filter(|table| !is_new_migration_table(*table))
        .filter(|table| !is_prisma_1_point_1_or_2_join_table(*table))
        .filter(|table| !is_prisma_1_point_0_join_table(*table))
        .filter(|table| !is_relay_table(*table))
    {
        debug!("Calculating model: {}", table.name());
        let mut model = Model::new(table.name().to_owned(), None);

        for column in table.columns() {
            let field = calculate_scalar_field(column, ctx);
            model.add_field(Field::ScalarField(field));
        }

        let duplicated_foreign_keys: HashSet<ForeignKeyId> = table
            .foreign_keys()
            .enumerate()
            .filter(|(idx, left)| {
                let mut already_visited = table.foreign_keys().take(*idx);
                already_visited.any(|right| {
                    let (left_constrained, right_constrained) =
                        (left.constrained_columns(), right.constrained_columns());
                    left_constrained.len() == right_constrained.len()
                        && left_constrained
                            .zip(right_constrained)
                            .all(|(left, right)| left.id == right.id)
                        && left
                            .referenced_columns()
                            .zip(right.referenced_columns())
                            .all(|(left, right)| left.id == right.id)
                })
            })
            .map(|(_, fk)| fk.id)
            .collect();

        for foreign_key in table
            .foreign_keys()
            .filter(|fk| !duplicated_foreign_keys.contains(&fk.id))
        {
            let mut relation_field = calculate_relation_field(foreign_key, &m2m_tables, &duplicated_foreign_keys);

            relation_field.supports_restrict_action(!ctx.sql_family().is_mssql());

            model.add_field(Field::RelationField(relation_field));
        }

        for index in table.indexes() {
            if let Some(index) = calculate_index(index, ctx) {
                model.add_index(index);
            }
        }

        if let Some(pk) = table.primary_key() {
            let clustered = primary_key_is_clustered(pk.id, ctx);

            model.primary_key = Some(PrimaryKeyDefinition {
                name: None,
                db_name: Some(pk.name().to_owned()),
                fields: pk
                    .columns()
                    .map(|c| {
                        let sort_order = c.sort_order().map(|sort| match sort {
                            SQLSortOrder::Asc => SortOrder::Asc,
                            SQLSortOrder::Desc => SortOrder::Desc,
                        });

                        PrimaryKeyField {
                            name: c.name().to_string(),
                            sort_order,
                            length: c.length(),
                        }
                    })
                    .collect(),
                defined_on_field: pk.columns().len() == 1,
                clustered,
            });
        }

        ctx.datamodel.add_model(model);
    }

    for e in schema.enums.iter() {
        let values = e.values.iter().map(|v| dml::EnumValue::new(v)).collect();
        ctx.datamodel.add_enum(dml::Enum::new(&e.name, values));
    }

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields
    for model in ctx.datamodel.models() {
        for relation_field in model.relation_fields() {
            let relation_info = &relation_field.relation_info;
            if ctx
                .datamodel
                .find_related_field_for_info(relation_info, &relation_field.name)
                .is_none()
            {
                let other_model = ctx.datamodel.find_model(&relation_info.to).unwrap();
                let field = calculate_backrelation_field(schema, model, other_model, relation_field, relation_info)?;

                fields_to_be_added.push((other_model.name.clone(), field));
            }
        }
    }

    // add prisma many to many relation fields
    for table in schema
        .table_walkers()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(*table) || is_prisma_1_point_0_join_table(*table))
    {
        calculate_fields_for_prisma_join_table(table, &mut fields_to_be_added, ctx)
    }

    for (model, field) in fields_to_be_added {
        ctx.datamodel
            .find_model_mut(&model)
            .add_field(Field::RelationField(field));
    }

    Ok(())
}

fn calculate_fields_for_prisma_join_table(
    join_table: TableWalker<'_>,
    fields_to_be_added: &mut Vec<(String, RelationField)>,
    ctx: &mut Context,
) {
    let mut foreign_keys = join_table.foreign_keys();
    if let (Some(fk_a), Some(fk_b)) = (foreign_keys.next(), foreign_keys.next()) {
        let is_self_relation = fk_a.referenced_table().id == fk_b.referenced_table().id;

        for (fk, opposite_fk) in &[(fk_a, fk_b), (fk_b, fk_a)] {
            let referenced_model = dml::find_model_by_db_name(ctx.datamodel, fk.referenced_table().name())
                .expect("Could not find model referenced in relation table.");

            let relation_name = join_table.name()[1..].to_owned();
            let field = calculate_many_to_many_field(*opposite_fk, relation_name, is_self_relation);

            fields_to_be_added.push((referenced_model.name.clone(), field));
        }
    }
}

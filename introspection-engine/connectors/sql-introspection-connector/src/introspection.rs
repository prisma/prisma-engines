use crate::{
    calculate_datamodel::CalculateDatamodelContext as Context,
    introspection_helpers::{
        calculate_backrelation_field, calculate_index, calculate_many_to_many_field, calculate_relation_field,
        calculate_scalar_field, is_new_migration_table, is_old_migration_table, is_prisma_1_point_0_join_table,
        is_prisma_1_point_1_or_2_join_table, is_relay_table, primary_key_is_clustered,
    },
    version_checker::VersionChecker,
    Dedup, SqlError, SqlFamilyTrait,
};
use datamodel::dml::{self, Field, Model, PrimaryKeyDefinition, PrimaryKeyField, RelationField, SortOrder};
use sql_schema_describer::{SQLSortOrder, Table};
use tracing::debug;

pub(crate) fn introspect(version_check: &mut VersionChecker, ctx: &mut Context) -> Result<(), SqlError> {
    let schema = ctx.schema;
    // collect m2m table names
    let m2m_tables: Vec<String> = schema
        .tables
        .iter()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(table) || is_prisma_1_point_0_join_table(table))
        .map(|table| table.name[1..].to_string())
        .collect();

    for table in schema
        .tables
        .iter()
        .filter(|table| !is_old_migration_table(table))
        .filter(|table| !is_new_migration_table(table))
        .filter(|table| !is_prisma_1_point_1_or_2_join_table(table))
        .filter(|table| !is_prisma_1_point_0_join_table(table))
        .filter(|table| !is_relay_table(table))
    {
        let walker = schema.table_walkers().find(|t| t.name() == table.name).unwrap();
        debug!("Calculating model: {}", table.name);
        let mut model = Model::new(table.name.clone(), None);

        for column in &table.columns {
            version_check.check_column_for_type_and_default_value(column);
            let field = calculate_scalar_field(table, column, ctx);
            model.add_field(Field::ScalarField(field));
        }

        let mut foreign_keys_copy = table.foreign_keys.clone();
        foreign_keys_copy.clear_duplicates();

        for foreign_key in &foreign_keys_copy {
            version_check.has_inline_relations(table);
            version_check.uses_on_delete(foreign_key, table);

            let mut relation_field = calculate_relation_field(schema, table, foreign_key, &m2m_tables)?;

            relation_field.supports_restrict_action(!ctx.sql_family().is_mssql());

            model.add_field(Field::RelationField(relation_field));
        }

        for index in walker.indexes() {
            model.add_index(calculate_index(index, ctx));
        }

        if let Some(pk) = &table.primary_key {
            let clustered = primary_key_is_clustered(walker.table_id(), ctx);

            model.primary_key = Some(PrimaryKeyDefinition {
                name: None,
                db_name: pk.constraint_name.clone(),
                fields: pk
                    .columns
                    .iter()
                    .map(|c| {
                        let sort_order = c.sort_order.map(|sort| match sort {
                            SQLSortOrder::Asc => SortOrder::Asc,
                            SQLSortOrder::Desc => SortOrder::Desc,
                        });

                        PrimaryKeyField {
                            name: c.name().to_string(),
                            sort_order,
                            length: c.length,
                        }
                    })
                    .collect(),
                defined_on_field: pk.columns.len() == 1,
                clustered,
            });
        }

        version_check.always_has_created_at_updated_at(table, &model);
        version_check.has_p1_compatible_primary_key_column(table);

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
        .tables
        .iter()
        .filter(|table| is_prisma_1_point_1_or_2_join_table(table) || is_prisma_1_point_0_join_table(table))
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
    join_table: &Table,
    fields_to_be_added: &mut Vec<(String, RelationField)>,
    ctx: &mut Context,
) {
    if let (Some(fk_a), Some(fk_b)) = (join_table.foreign_keys.get(0), join_table.foreign_keys.get(1)) {
        let is_self_relation = fk_a.referenced_table == fk_b.referenced_table;

        for (fk, opposite_fk) in &[(fk_a, fk_b), (fk_b, fk_a)] {
            let referenced_model = dml::find_model_by_db_name(ctx.datamodel, &fk.referenced_table)
                .expect("Could not find model referenced in relation table.");

            let relation_name = join_table.name[1..].to_string();
            let field = calculate_many_to_many_field(opposite_fk, relation_name, is_self_relation);

            fields_to_be_added.push((referenced_model.name.clone(), field));
        }
    }
}

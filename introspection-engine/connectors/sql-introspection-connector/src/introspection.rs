use crate::introspection_helpers::{
    calculate_backrelation_field, calculate_index, calculate_many_to_many_field, calculate_relation_field,
    calculate_scalar_field, is_new_migration_table, is_old_migration_table, is_prisma_1_point_0_join_table,
    is_prisma_1_point_1_or_2_join_table, is_relay_table,
};
use crate::version_checker::VersionChecker;

use crate::{Dedup, SqlError, SqlFamilyTrait};
use datamodel::{
    common::preview_features::PreviewFeature,
    dml::{self, Datamodel, Field, Model, PrimaryKeyDefinition, PrimaryKeyField, RelationField, SortOrder},
};
use introspection_connector::IntrospectionContext;
use sql_schema_describer::{SQLSortOrder, SqlSchema, Table};
use tracing::debug;

pub fn introspect(
    schema: &SqlSchema,
    version_check: &mut VersionChecker,
    data_model: &mut Datamodel,
    ctx: &IntrospectionContext,
) -> Result<(), SqlError> {
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

        for index in &table.indices {
            // TODO: enable with preview flag, when dml index extensions are done.
            if !ctx.preview_features.contains(PreviewFeature::ExtendedIndexes)
                && index.columns.iter().any(|c| c.length.is_some())
            {
                continue;
            }

            model.add_index(calculate_index(index, ctx));
        }

        if let Some(pk) = &table.primary_key {
            model.primary_key = Some(PrimaryKeyDefinition {
                name: None,
                db_name: pk.constraint_name.clone(),
                fields: pk
                    .columns
                    .iter()
                    .map(|c| {
                        let (sort_order, length) = if !ctx.preview_features.contains(PreviewFeature::ExtendedIndexes) {
                            (None, None)
                        } else {
                            let sort_order = c.sort_order.map(|sort| match sort {
                                SQLSortOrder::Asc => SortOrder::Asc,
                                SQLSortOrder::Desc => SortOrder::Desc,
                            });
                            (sort_order, c.length)
                        };

                        PrimaryKeyField {
                            name: c.name().to_string(),
                            sort_order,
                            length,
                        }
                    })
                    .collect(),
                defined_on_field: pk.columns.len() == 1,
                clustered: true,
            });
        }

        version_check.always_has_created_at_updated_at(table, &model);
        version_check.has_p1_compatible_primary_key_column(table);

        data_model.add_model(model);
    }

    for e in schema.enums.iter() {
        let values = e.values.iter().map(|v| dml::EnumValue::new(v)).collect();
        data_model.add_enum(dml::Enum::new(&e.name, values));
    }

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields
    for model in data_model.models() {
        for relation_field in model.relation_fields() {
            let relation_info = &relation_field.relation_info;
            if data_model
                .find_related_field_for_info(relation_info, &relation_field.name)
                .is_none()
            {
                let other_model = data_model.find_model(&relation_info.to).unwrap();
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
        calculate_fields_for_prisma_join_table(table, &mut fields_to_be_added, data_model)
    }

    for (model, field) in fields_to_be_added {
        data_model.find_model_mut(&model).add_field(Field::RelationField(field));
    }

    Ok(())
}

fn calculate_fields_for_prisma_join_table(
    join_table: &Table,
    fields_to_be_added: &mut Vec<(String, RelationField)>,
    data_model: &mut Datamodel,
) {
    if let (Some(fk_a), Some(fk_b)) = (join_table.foreign_keys.get(0), join_table.foreign_keys.get(1)) {
        let is_self_relation = fk_a.referenced_table == fk_b.referenced_table;

        for (fk, opposite_fk) in &[(fk_a, fk_b), (fk_b, fk_a)] {
            let referenced_model = dml::find_model_by_db_name(data_model, &fk.referenced_table)
                .expect("Could not find model referenced in relation table.");

            let relation_name = join_table.name[1..].to_string();
            let field = calculate_many_to_many_field(opposite_fk, relation_name, is_self_relation);

            fields_to_be_added.push((referenced_model.name.clone(), field));
        }
    }
}

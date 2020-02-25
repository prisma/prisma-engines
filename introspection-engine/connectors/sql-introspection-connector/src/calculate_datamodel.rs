use crate::comment_out_unhandled_models::comment_out_unhandled_models;
use crate::misc_helpers::*;
use crate::sanitize_datamodel_names::sanitize_datamodel_names;
use crate::SqlIntrospectionResult;
use datamodel::{dml, Datamodel, FieldType, Model};
use log::debug;
use sql_schema_describer::*;
use std::collections::HashSet;

/// Calculate a data model from a database schema.
pub fn calculate_model(schema: &SqlSchema) -> SqlIntrospectionResult<Datamodel> {
    debug!("Calculating data model.");

    let mut data_model = Datamodel::new();
    for table in schema
        .tables
        .iter()
        .filter(|table| !is_migration_table(&table))
        .filter(|table| !is_prisma_join_table(&table))
    {
        debug!("Calculating model: {}", table.name);
        let mut model = Model::new(table.name.clone(), None);

        for column in table
            .columns
            .iter()
            .filter(|column| !is_foreign_key_column(&table, &column))
        {
            let field = calculate_scalar_field(&schema, &table, &column, None);
            model.add_field(field);
        }

        for foreign_key in &table.foreign_keys {
            let mut fields = calculate_relation_field(schema, table, foreign_key, &table.foreign_keys);
            model.add_fields(&mut fields);
        }

        for index in &table.indices {
            let fk_on_index = table.foreign_keys.iter().find(|fk| fk.columns == index.columns);

            // This is part of the temporary guardrail that makes us ignore relations that overlap
            // with the primary key.
            let fk_on_index_overlaps_with_pk: bool = table
                .primary_key
                .as_ref()
                .and_then(|pk| fk_on_index.as_ref().map(|fk| (pk, fk)))
                .map(|(pk, fk)| {
                    pk.columns
                        .iter()
                        .any(|pk_col| fk.columns.iter().any(|fk_col| pk_col == fk_col))
                })
                .unwrap_or(false);

            // The database names of relation fields and the index columns are not necessarily in
            // the same order, so a simple equality check doesn't work.
            let index_columns: HashSet<&String> = index.columns.iter().collect();

            let compound_field_name = || {
                model
                    .fields
                    .iter()
                    .find(|f| {
                        !f.database_names.is_empty()
                            && f.database_names
                                .iter()
                                .all(|database_field_name| index_columns.contains(database_field_name))
                    })
                    .expect("Error finding field matching a compound index.")
                    .name
                    .clone()
            };

            let index_to_add = match (fk_on_index, index.columns.len(), index.is_unique()) {
                (Some(_), _, true) => None, // just make the relation 1:1 and dont print the unique index
                (Some(_), 1, false) => Some(calculate_index(index)),
                (Some(_), _, false) if !fk_on_index_overlaps_with_pk => {
                    Some(calculate_compound_index(index, compound_field_name()))
                }
                (Some(_), _, false) => Some(calculate_index(index)),
                (None, 1, true) => None, // this is expressed by the @unique already
                (None, _, true) => Some(calculate_index(index)),
                (None, _, false) => Some(calculate_index(index)),
            };

            index_to_add.map(|i| model.add_index(i));
        }

        if table.primary_key_columns().len() > 1 {
            model.id_fields = table.primary_key_columns();
        }

        data_model.add_model(model);
    }

    for e in schema.enums.iter() {
        data_model.add_enum(dml::Enum {
            name: e.name.clone(),
            values: e.values.iter().map(|v| dml::EnumValue::new(v, None)).collect(),
            database_name: None,
            documentation: None,
        });
    }

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields
    for model in data_model.models.iter() {
        for relation_field in model.fields.iter() {
            if let FieldType::Relation(relation_info) = &relation_field.field_type {
                if data_model
                    .related_field(
                        &model.name,
                        &relation_info.to,
                        &relation_info.name,
                        &relation_field.name,
                    )
                    .is_none()
                {
                    let other_model = data_model.find_model(relation_info.to.as_str()).unwrap();
                    let field = calculate_backrelation_field(schema, model, other_model, relation_field, relation_info);

                    fields_to_be_added.push((other_model.name.clone(), field));
                }
            }
        }
    }

    // add prisma many to many relation fields
    for table in schema.tables.iter().filter(|table| is_prisma_join_table(&table)) {
        if let (Some(f), Some(s)) = (table.foreign_keys.get(0), table.foreign_keys.get(1)) {
            let is_self_relation = f.referenced_table == s.referenced_table;

            fields_to_be_added.push((
                s.referenced_table.clone(),
                calculate_many_to_many_field(f, table.name[1..].to_string(), is_self_relation),
            ));
            fields_to_be_added.push((
                f.referenced_table.clone(),
                calculate_many_to_many_field(s, table.name[1..].to_string(), is_self_relation),
            ));
        }
    }

    deduplicate_names_of_fields_to_be_added(&mut fields_to_be_added);

    for (model, field) in fields_to_be_added {
        let model = data_model.find_model_mut(&model).unwrap();
        model.add_field(field);
    }

    comment_out_unhandled_models(&mut data_model);
    sanitize_datamodel_names(&mut data_model);
    debug!("Done calculating data model {:?}", data_model);

    Ok(data_model)
}

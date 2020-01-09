use crate::misc_helpers::*;
use crate::sanitize_datamodel_names::sanitize_datamodel_names;
use crate::SqlIntrospectionResult;
use datamodel::{
    common::names::NameNormalizer,
    dml,
    DatabaseName::{Compound, Single},
    Datamodel, Field, FieldArity, FieldType, IndexDefinition, Model, OnDeleteStrategy, RelationInfo,
};
use log::debug;
use prisma_inflector;
use sql_schema_describer::*;

/// Calculate a data model from a database schema.
pub fn calculate_model(schema: &SqlSchema) -> SqlIntrospectionResult<Datamodel> {
    debug!("Calculating data model");

    let mut data_model = Datamodel::new();
    for table in schema
        .tables
        .iter()
        .filter(|table| !is_migration_table(&table))
        .filter(|table| !is_prisma_join_table(&table))
    {
        let mut model = Model::new(table.name.clone(), None);

        for column in table
            .columns
            .iter()
            .filter(|column| !is_compound_foreign_key_column(&table, &column))
        {
            //todo add non-compound fields
            debug!("Handling column {:?}", column);
            let field_type = calculate_field_type(&schema, &column, &table);
            let arity = match column.tpe.arity {
                ColumnArity::Required => FieldArity::Required,
                ColumnArity::Nullable => FieldArity::Optional,
                ColumnArity::List => FieldArity::List,
            };
            let id_info = calculate_id_info(&column, &table);
            let default_value = match field_type {
                FieldType::Relation(_) => None,
                _ if arity == FieldArity::List => None,
                _ => column
                    .default
                    .as_ref()
                    .and_then(|default| calculate_default(default, &column.tpe.family)),
            };

            let is_unique = match field_type {
                datamodel::dml::FieldType::Relation(..) => false,
                _ if id_info.is_some() => false,
                _ => table.is_column_unique(&column.name),
            };

            let field = Field {
                name: column.name.clone(),
                arity,
                field_type,
                database_name: None,
                default_value,
                is_unique,
                id_info,
                documentation: None,
                is_generated: false,
                is_updated_at: false,
            };

            model.add_field(field);
        }

        //do not add compound indexes to schema when they cover a foreign key, instead make the relation 1:1
        for index in table.indices.iter().filter(|i| {
            table
                .foreign_keys
                .iter()
                .all(|fk| !is_foreign_key_covered_by_unique_index(i, fk))
        }) {
            debug!("Handling index  {:?}", index);
            let tpe = match index.tpe {
                IndexType::Unique => datamodel::dml::IndexType::Unique,
                IndexType::Normal => datamodel::dml::IndexType::Normal,
            };

            let index_definition: IndexDefinition = IndexDefinition {
                name: Some(index.name.clone()),
                fields: index.columns.clone(),
                tpe,
            };

            match (index.columns.len(), &index.tpe) {
                (1, IndexType::Unique) => (), // they go on the field not the model in the datamodel
                _ => model.add_index(index_definition),
            }
        }

        //add compound fields
        for foreign_key in table.foreign_keys.iter().filter(|fk| fk.columns.len() > 1) {
            debug!("Handling compound foreign key  {:?}", foreign_key);

            let field_type = FieldType::Relation(RelationInfo {
                name: calculate_relation_name(schema, foreign_key, table),
                to: foreign_key.referenced_table.clone(),
                to_fields: foreign_key.referenced_columns.clone(),
                on_delete: OnDeleteStrategy::None,
            });

            let columns: Vec<&Column> = foreign_key
                .columns
                .iter()
                .map(|c| table.columns.iter().find(|tc| tc.name == *c).unwrap())
                .collect();

            let arity = match columns.iter().find(|c| c.is_required()).is_none() {
                true => FieldArity::Optional,
                false => FieldArity::Required,
            };

            // todo this later needs to be a compound value of the two columns defaults?
            let default_value = None;

            let is_unique = false;

            //todo name of the opposing model  -> still needs to be sanitized
            let name = foreign_key.referenced_table.clone().camel_case();

            let database_name = Some(Compound(columns.iter().map(|c| c.name.clone()).collect()));

            let field = Field {
                name,
                arity,
                field_type,
                database_name,
                default_value,
                is_unique,
                id_info: None,
                documentation: None,
                is_generated: false,
                is_updated_at: false,
            };

            model.add_field(field);
        }

        if table.primary_key_columns().len() > 1 {
            model.id_fields = table.primary_key_columns();
        }

        data_model.add_model(model);
    }

    for e in schema.enums.iter() {
        let mut values: Vec<String> = e.values.iter().cloned().collect();
        values.sort_unstable();
        data_model.add_enum(dml::Enum {
            name: e.name.clone(),
            values,
            database_name: None,
            documentation: None,
        });
    }

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields
    for model in data_model.models.iter() {
        for relation_field in model.fields.iter() {
            match &relation_field.field_type {
                FieldType::Relation(relation_info) => {
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

                        let table = schema.table_bang(model.name.as_str());
                        let fk = table.foreign_key_for_column(relation_field.name.as_str());
                        let on_delete = match fk {
                            Some(fk) if fk.on_delete_action == ForeignKeyAction::Cascade => OnDeleteStrategy::Cascade,
                            _ => OnDeleteStrategy::None,
                        };

                        let field_type = FieldType::Relation(RelationInfo {
                            name: relation_info.name.clone(),
                            to: model.name.clone(),
                            to_fields: vec![relation_field.name.clone()],
                            on_delete,
                        });

                        //todo
                        // fetch this from indexes
                        // what about separate uniques? all @unique == @@unique ?? No! separate ones do not fully work since you can only connect to a subset of the @@unique case
                        // model.indexes contains a multi-field unique index that matches the colums exactly, then it is unique
                        // if there are separate uniques it probably should not become a relation
                        // what breaks by having an @@unique that refers to fields that do not have a representation on the model anymore due to the merged relation field?

                        let other_is_unique = || {
                            let table = schema.table_bang(&model.name);

                            match &relation_field.database_name {
                                None => table.is_column_unique(relation_field.name.as_str()),
                                Some(Single(name)) => table.is_column_unique(name),
                                Some(Compound(names)) => table.indices.iter().any(|i| i.columns == *names),
                            }
                        };

                        let arity = match relation_field.arity {
                            FieldArity::Required | FieldArity::Optional if other_is_unique() => FieldArity::Optional,
                            FieldArity::Required | FieldArity::Optional => FieldArity::List,
                            FieldArity::List => FieldArity::Optional,
                        };

                        let inflector = prisma_inflector::default();

                        let name = match arity {
                            FieldArity::List => inflector.pluralize(&model.name).camel_case(), // pluralize
                            FieldArity::Optional => model.name.clone().camel_case(),
                            FieldArity::Required => model.name.clone().camel_case(),
                        };

                        let field = Field {
                            name,
                            arity,
                            field_type,
                            database_name: None,
                            default_value: None,
                            is_unique: false,
                            id_info: None,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                        };

                        fields_to_be_added.push((other_model.name.clone(), field));
                    }
                }
                _ => {}
            }
        }
    }

    // add prisma many to many relation fields
    for table in schema.tables.iter().filter(|table| is_prisma_join_table(&table)) {
        let first = table.foreign_keys.get(0);
        let second = table.foreign_keys.get(1);

        match (first, second) {
            (Some(f), Some(s)) => {
                let is_self_relation = f.referenced_table == s.referenced_table;

                fields_to_be_added.push((
                    s.referenced_table.clone(),
                    create_many_to_many_field(f, table.name[1..].to_string(), is_self_relation),
                ));
                fields_to_be_added.push((
                    f.referenced_table.clone(),
                    create_many_to_many_field(s, table.name[1..].to_string(), is_self_relation),
                ));
            }
            (_, _) => (),
        }
    }

    //todo make separate method: find duplicated field indexes
    let mut duplicated_relation_fields = Vec::new();
    fields_to_be_added
        .iter()
        .enumerate()
        .for_each(|(index, (model, field))| {
            let is_duplicated = fields_to_be_added
                .iter()
                .filter(|(other_model, other_field)| model == other_model && field.name == other_field.name)
                .count()
                > 1;

            if is_duplicated {
                duplicated_relation_fields.push(index);
            }
        });

    //todo make separate method: disambiguate names
    duplicated_relation_fields.iter().for_each(|index| {
        let (_, ref mut field) = fields_to_be_added.get_mut(*index).unwrap();
        let suffix = match &field.field_type {
            FieldType::Relation(RelationInfo { name, .. }) => format!("_{}", &name),
            FieldType::Base(_) => "".to_string(),
            _ => "".to_string(),
        };

        field.name = format!("{}{}", field.name, suffix)
    });

    for (model, field) in fields_to_be_added {
        let model = data_model.find_model_mut(&model).unwrap();
        model.add_field(field);
    }

    Ok(sanitize_datamodel_names(data_model))
}

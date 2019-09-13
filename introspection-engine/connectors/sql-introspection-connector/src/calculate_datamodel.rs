use crate::SqlIntrospectionResult;
use datamodel::{
    common::{names::NameNormalizer, PrismaType, PrismaValue},
    dml, Datamodel, Field, FieldArity, FieldType, IdInfo, IdStrategy, Model, OnDeleteStrategy, RelationInfo,
    ScalarListStrategy,
};
use log::debug;
use prisma_inflector;
use regex::Regex;
use sql_schema_describer::*;

pub fn is_migration_table(table: &Table) -> bool {
    table.name == "_Migration"
}

pub fn is_many_to_many_relation_table(table: &Table) -> bool {
    table.name.starts_with("_")
        && table.columns.iter().count() == 2
        && table.columns.iter().find(|column| column.name == "A").is_some()
        && table.columns.iter().find(|column| column.name == "B").is_some()
}

pub fn is_scalar_list_table(table: &Table) -> bool {
    table.name.contains("_")
        && table.columns.iter().count() == 3
        && table.columns.iter().find(|column| column.name == "nodeId").is_some()
        && table.columns.iter().find(|column| column.name == "position").is_some()
        && table.columns.iter().find(|column| column.name == "value").is_some()
}

/// Calculate a data model from a database schema.
pub fn calculate_model(schema: &SqlSchema) -> SqlIntrospectionResult<Datamodel> {
    debug!("Calculating data model");

    let mut data_model = Datamodel::new();
    for table in schema
        .tables
        .iter()
        .filter(|table| !is_migration_table(&table))
        // .filter(|table| !is_many_to_many_relation_table(&table))
    .filter(|table| !is_scalar_list_table(&table))
    {
        let mut model = Model::new(&table.name);
        for column in table.columns.iter() {
            debug!("Handling column {:?}", column);
            let field_type = calculate_field_type(&column, &table);
            let arity = match column.arity {
                ColumnArity::Required => FieldArity::Required,
                ColumnArity::Nullable => FieldArity::Optional,
                ColumnArity::List => FieldArity::List,
            };
            let id_info = calc_id_info(&column, &table);
            let scalar_list_strategy = match arity {
                FieldArity::List => Some(ScalarListStrategy::Embedded),
                _ => None,
            };
            let default_value = column
                .default
                .as_ref()
                .and_then(|default| calculate_default(default, &column.tpe.family));

            let is_unique = if id_info.is_some() {
                false
            } else {
                table.is_column_unique(&column)
            };
            let field = Field {
                name: column.name.clone(),
                arity,
                field_type,
                database_name: None,
                default_value,
                is_unique: is_unique,
                id_info,
                scalar_list_strategy,
                documentation: None,
                is_generated: false,
                is_updated_at: false,
            };
            model.add_field(field);
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

    // add backrelation fields
    let mut back_relation_fields = Vec::new();

    for model in data_model.models.iter() {
        for relation_field in model.fields.iter() {
            match &relation_field.field_type {
                FieldType::Relation(relation_info) => {
                    if data_model
                        .related_field_new(
                            &model.name,
                            &relation_info.to,
                            &relation_info.name,
                            &relation_field.name,
                        )
                        .is_none()
                    {
                        let other_model = data_model.find_model(&relation_info.to).unwrap();

                        let field_type = FieldType::Relation(RelationInfo {
                            name: "".to_string(),
                            to: model.name.clone(),
                            to_fields: vec![],
                            on_delete: OnDeleteStrategy::None,
                        });

                        let arity = match relation_field.arity {
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
                            scalar_list_strategy: None,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                        };

                        back_relation_fields.push((other_model.name.clone(), field));
                    }
                }
                _ => {}
            }
        }
    }

    for (model, field) in back_relation_fields {
        let model = data_model.find_model_mut(&model).unwrap();
        model.add_field(field);
    }

    // add many to many relation fields
    // let mut many_to_many_relation_fields = Vec::new();
    // for table in schema.tables.iter().filter(|table| is_many_to_many_relation_table(&table)){
            // use foreign keys to find tables that are linked to??

    //     many_to_many_relation_fields.push((model.clone(), field1 ));   
    //     many_to_many_relation_fields.push((model.clone(), field2 ));   
    // }

    //  for (model, field) in many_to_many_relation_fields {
    //     let model = data_model.find_model_mut(&model).unwrap();
    //     model.add_field(field);
    // }

    // add scalar lists fields
    let mut scalar_list_fields = Vec::new();
    for table in schema.tables.iter().filter(|table| is_scalar_list_table(&table)){
        let name = table.name.split('_').nth(1).unwrap();
        let model = table.name.split('_').nth(0).unwrap();

        let field_type = calculate_field_type(&table.columns.iter().find(|c| c.name == "value").unwrap(), &table);

        let field = Field {
                            name: name.to_string(),
                            arity: FieldArity::List,
                            field_type,  
                            database_name: None,
                            default_value: None,
                            is_unique: false,
                            id_info: None,
                            scalar_list_strategy: None,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                        };
    
         scalar_list_fields.push((model.clone(), field ));   
    }

     for (model, field) in scalar_list_fields {
        let model = data_model.find_model_mut(&model).unwrap();
        model.add_field(field);
    }

    Ok(data_model)
}

fn parse_int(value: &str) -> Option<i32> {
    debug!("Parsing int '{}'", value);
    let re_num = Regex::new(r"^'?(\d+)'?$").expect("compile regex");
    let rslt = re_num.captures(value);
    if rslt.is_none() {
        debug!("Couldn't parse int");
        return None;
    }

    let captures = rslt.expect("get captures");
    let num_str = captures.get(1).expect("get capture").as_str();
    let num_rslt = num_str.parse::<i32>();
    match num_rslt {
        Ok(num) => Some(num),
        Err(_) => {
            debug!("Couldn't parse int '{}'", num_str);
            None
        }
    }
}

fn parse_float(value: &str) -> Option<f32> {
    debug!("Parsing float '{}'", value);
    let re_num = Regex::new(r"^'?([^']+)'?$").expect("compile regex");
    let rslt = re_num.captures(value);
    if rslt.is_none() {
        debug!("Couldn't parse float");
        return None;
    }

    let captures = rslt.expect("get captures");
    let num_str = captures.get(1).expect("get capture").as_str();
    let num_rslt = num_str.parse::<f32>();
    match num_rslt {
        Ok(num) => Some(num),
        Err(_) => {
            debug!("Couldn't parse float '{}'", num_str);
            None
        }
    }
}

fn calculate_default(default: &str, tpe: &ColumnTypeFamily) -> Option<PrismaValue> {
    match tpe {
        ColumnTypeFamily::Boolean => parse_int(default).map(|x| PrismaValue::Boolean(x != 0)),
        ColumnTypeFamily::Int => parse_int(default).map(|x| PrismaValue::Int(x)),
        ColumnTypeFamily::Float => parse_float(default).map(|x| PrismaValue::Float(x)),
        ColumnTypeFamily::String => Some(PrismaValue::String(default.to_string())),
        _ => None,
    }
}

fn calc_id_info(column: &Column, table: &Table) -> Option<IdInfo> {
    table.primary_key.as_ref().and_then(|pk| {
        if pk.contains_column(&column.name) {
            let strategy = match column.auto_increment {
                true => IdStrategy::Auto,
                false => IdStrategy::None,
            };
            Some(IdInfo {
                strategy,
                sequence: pk.sequence.as_ref().map(|sequence| dml::Sequence {
                    name: sequence.name.clone(),
                    allocation_size: sequence.allocation_size as i32,
                    initial_value: sequence.initial_value as i32,
                }),
            })
        } else {
            None
        }
    })
}

fn calculate_field_type(column: &Column, table: &Table) -> FieldType {
    debug!("Calculating field type for '{}'", column.name);
    // Look for a foreign key referencing this column
    match table.foreign_keys.iter().find(|fk| fk.columns.contains(&column.name)) {
        Some(fk) => {
            debug!("Found corresponding foreign key");
            let idx = fk
                .columns
                .iter()
                .position(|n| n == &column.name)
                .expect("get column FK position");
            let referenced_col = &fk.referenced_columns[idx];
            FieldType::Relation(RelationInfo {
                name: "".to_string(),
                to: fk.referenced_table.clone(),
                to_fields: vec![referenced_col.clone()],
                on_delete: match fk.on_delete_action {
                    ForeignKeyAction::Cascade => OnDeleteStrategy::Cascade,
                    _ => OnDeleteStrategy::None,
                },
            })
        }
        None => {
            debug!("Found no corresponding foreign key");
            match column.tpe.family {
                ColumnTypeFamily::Boolean => FieldType::Base(PrismaType::Boolean),
                ColumnTypeFamily::DateTime => FieldType::Base(PrismaType::DateTime),
                ColumnTypeFamily::Float => FieldType::Base(PrismaType::Float),
                ColumnTypeFamily::Int => FieldType::Base(PrismaType::Int),
                ColumnTypeFamily::String => FieldType::Base(PrismaType::String),
                // XXX: We made a conscious decision to punt on mapping of ColumnTypeFamily
                // variants that don't yet have corresponding PrismaType variants
                _ => FieldType::Base(PrismaType::String),
            }
        }
    }
}

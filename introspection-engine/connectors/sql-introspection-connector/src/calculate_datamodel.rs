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

fn is_migration_table(table: &Table) -> bool {
    table.name == "_Migration"
}

fn is_prisma_join_table(table: &Table) -> bool {
    table.columns.iter().count() == 2
        && table.foreign_keys.iter().count() == 2
        && table.name.starts_with("_")
        && table.columns.iter().find(|column| column.name == "A").is_some()
        && table.columns.iter().find(|column| column.name == "B").is_some()
}

fn is_prisma_scalar_list_table(table: &Table) -> bool {
    table.name.contains("_")
        && table.columns.iter().count() == 3
        && table.columns.iter().find(|column| column.name == "nodeId").is_some()
        && table.columns.iter().find(|column| column.name == "position").is_some()
        && table.columns.iter().find(|column| column.name == "value").is_some()
}

fn create_many_to_many_field(foreign_key: &ForeignKey, relation_name: String, is_self_relation: bool) -> Field {
    let inflector = prisma_inflector::default();

    let field_type = FieldType::Relation(RelationInfo {
        name: relation_name,
        to: foreign_key.referenced_table.clone(),
        to_fields: foreign_key.referenced_columns.clone(),
        on_delete: OnDeleteStrategy::None,
    });

    let basename = inflector.pluralize(&foreign_key.referenced_table).camel_case();

    let name = match is_self_relation {
        true => format!("{}_{}", basename, foreign_key.columns[0]),
        false => basename,
    };

    Field {
        name,
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
    }
}

/// Calculate a data model from a database schema.
pub fn calculate_model(schema: &SqlSchema) -> SqlIntrospectionResult<Datamodel> {
    debug!("Calculating data model");

    let mut data_model = Datamodel::new();
    for table in schema
        .tables
        .iter()
        .filter(|table| !is_migration_table(&table))
        .filter(|table| !is_prisma_join_table(&table))
        .filter(|table| !is_prisma_scalar_list_table(&table))
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

    let mut fields_to_be_added = Vec::new();

    // add backrelation fields

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
                            name: relation_info.name.clone(),
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
                    create_many_to_many_field(f, table.name.clone() , is_self_relation),
                ));
                fields_to_be_added.push((
                    f.referenced_table.clone(),
                    create_many_to_many_field(s, table.name.clone() , is_self_relation),
                ));
            }
            (_, _) => (),
        }
    }

    // add scalar lists fields
    for table in schema.tables.iter().filter(|table| is_prisma_scalar_list_table(&table)) {
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
            scalar_list_strategy: Some(ScalarListStrategy::Relation),
            documentation: None,
            is_generated: false,
            is_updated_at: false,
        };

        fields_to_be_added.push((model.to_owned(), field));
    }

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

fn not_many_to_many_relation_name(fk: &ForeignKey, table: &Table) -> String {
    let referenced_model = &fk.referenced_table;
    let model_with_fk = &table.name;
    let fk_column_name = fk.columns.get(0).unwrap();

    if model_with_fk < referenced_model {
        format!("{}_{}To{}", model_with_fk, fk_column_name, referenced_model)
    } else {
        format!("{}To{}_{}", referenced_model, model_with_fk, fk_column_name)
    }
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
                name: not_many_to_many_relation_name(fk, table),
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

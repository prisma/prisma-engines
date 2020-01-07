use datamodel::common::names::NameNormalizer;
use datamodel::{
    dml, Field, FieldArity, FieldType, IdInfo, IdStrategy, OnDeleteStrategy, RelationInfo, ScalarType, ScalarValue,
};
use log::debug;
use regex::Regex;
use sql_schema_describer::{Column, ColumnTypeFamily, ForeignKey, Index, IndexType, SqlSchema, Table};

//checks

pub fn is_foreign_key_covered_by_unique_index(index: &Index, foreign_key: &ForeignKey) -> bool {
    match index.tpe {
        IndexType::Unique => foreign_key.columns == index.columns,
        IndexType::Normal => false,
    }
}

pub fn is_migration_table(table: &Table) -> bool {
    table.name == "_Migration"
}

pub(crate) fn is_prisma_join_table(table: &Table) -> bool {
    table.columns.len() == 2
        && table.foreign_keys.len() == 2
        && table.foreign_keys[0].referenced_table < table.foreign_keys[1].referenced_table
        && table.name.starts_with("_")
        && table
            .columns
            .iter()
            .find(|column| column.name.to_lowercase() == "a")
            .is_some()
        && table
            .columns
            .iter()
            .find(|column| column.name.to_lowercase() == "b")
            .is_some()
        && table.indices.len() == 1
        && table.indices[0].columns.len() == 2
        && table.indices[0].tpe == IndexType::Unique
}

pub fn create_many_to_many_field(foreign_key: &ForeignKey, relation_name: String, is_self_relation: bool) -> Field {
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
        documentation: None,
        is_generated: false,
        is_updated_at: false,
    }
}

pub(crate) fn is_compound_foreign_key_column(table: &Table, column: &Column) -> bool {
    match table.foreign_keys.iter().find(|fk| fk.columns.contains(&column.name)) {
        Some(fk) if fk.columns.len() > 1 => true,
        _ => false,
    }
}

//calculators

pub(crate) fn calculate_default(default: &str, tpe: &ColumnTypeFamily) -> Option<ScalarValue> {
    match tpe {
        ColumnTypeFamily::Boolean => match parse_int(default) {
            Some(x) => Some(ScalarValue::Boolean(x != 0)),
            None => parse_bool(default).map(|b| ScalarValue::Boolean(b)),
        },
        ColumnTypeFamily::Int => parse_int(default).map(|x| ScalarValue::Int(x)),
        ColumnTypeFamily::Float => parse_float(default).map(|x| ScalarValue::Float(x)),
        ColumnTypeFamily::String => Some(ScalarValue::String(default.to_string())),
        _ => None,
    }
}

pub(crate) fn calculate_id_info(column: &Column, table: &Table) -> Option<IdInfo> {
    table.primary_key.as_ref().and_then(|pk| {
        if pk.is_single_primary_key(&column.name) {
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

pub(crate) fn calculate_relation_name(schema: &SqlSchema, fk: &ForeignKey, table: &Table) -> String {
    //this is not called for prisma many to many relations. for them the name is just the name of the join table.
    let referenced_model = &fk.referenced_table;
    let model_with_fk = &table.name;
    let fk_column_name = fk.columns.get(0).unwrap();

    let fk_to_same_model: Vec<&ForeignKey> = table
        .foreign_keys
        .iter()
        .filter(|fk| fk.referenced_table == referenced_model.clone())
        .collect();

    let fk_from_other_model_to_this: Vec<&ForeignKey> = schema
        .table_bang(referenced_model)
        .foreign_keys
        .iter()
        .filter(|fk| fk.referenced_table == model_with_fk.clone())
        .collect();

    //unambiguous
    if fk_to_same_model.len() < 2 && fk_from_other_model_to_this.len() == 0 {
        if model_with_fk < referenced_model {
            format!("{}To{}", model_with_fk, referenced_model)
        } else {
            format!("{}To{}", referenced_model, model_with_fk)
        }
    } else {
        //ambiguous
        if model_with_fk < referenced_model {
            format!("{}_{}To{}", model_with_fk, fk_column_name, referenced_model)
        } else {
            format!("{}To{}_{}", referenced_model, model_with_fk, fk_column_name)
        }
    }
}

pub(crate) fn calculate_field_type(schema: &SqlSchema, column: &Column, table: &Table) -> FieldType {
    debug!("Calculating field type for '{}'", column.name);
    // Look for a foreign key referencing this column
    match table.foreign_keys.iter().find(|fk| fk.columns.contains(&column.name)) {
        Some(fk) if calculate_id_info(column, table).is_none() => {
            debug!("Found corresponding foreign key");
            let idx = fk
                .columns
                .iter()
                .position(|n| n == &column.name)
                .expect("get column FK position");
            let referenced_col = &fk.referenced_columns[idx];

            FieldType::Relation(RelationInfo {
                name: calculate_relation_name(schema, fk, table),
                to: fk.referenced_table.clone(),
                to_fields: vec![referenced_col.clone()],
                on_delete: OnDeleteStrategy::None,
            })
        }
        _ => {
            debug!("Found no corresponding foreign key");
            match column.tpe.family {
                ColumnTypeFamily::Boolean => FieldType::Base(ScalarType::Boolean),
                ColumnTypeFamily::DateTime => FieldType::Base(ScalarType::DateTime),
                ColumnTypeFamily::Float => FieldType::Base(ScalarType::Float),
                ColumnTypeFamily::Int => FieldType::Base(ScalarType::Int),
                ColumnTypeFamily::String => FieldType::Base(ScalarType::String),
                // XXX: We made a conscious decision to punt on mapping of ColumnTypeFamily
                // variants that don't yet have corresponding PrismaType variants
                _ => FieldType::Base(ScalarType::String),
            }
        }
    }
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

fn parse_bool(value: &str) -> Option<bool> {
    debug!("Parsing bool '{}'", value);
    value.to_lowercase().parse().ok()
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

use datamodel::common::names::NameNormalizer;
use datamodel::{
    DefaultValue as DMLDef, Field, FieldArity, FieldType, IndexDefinition, Model, OnDeleteStrategy,
    RelationInfo, ScalarType, ScalarValue as SV, ValueGenerator as VG,
};
use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;
use sql_schema_describer::{
    Column, ColumnArity, ColumnTypeFamily, DefaultValue as SQLDef, ForeignKey, Index, IndexType,
    SqlSchema, Table,
};

//checks

pub fn is_migration_table(table: &Table) -> bool {
    table.name == "_Migration"
}

pub(crate) fn is_prisma_1_point_1_join_table(table: &Table) -> bool {
    table.columns.len() == 2
        && table.indices.len() >= 2
        && common_prisma_m_to_n_relation_conditions(table)
}

pub(crate) fn is_prisma_1_point_0_join_table(table: &Table) -> bool {
    table.columns.len() == 3
        && table.indices.len() >= 2
        && table.columns.iter().any(|c| c.name.as_str() == "id")
        && common_prisma_m_to_n_relation_conditions(table)
}

fn common_prisma_m_to_n_relation_conditions(table: &Table) -> bool {
    fn is_a(column: &String) -> bool {
        column.to_lowercase() == "a"
    }

    fn is_b(column: &String) -> bool {
        column.to_lowercase() == "b"
    }

    table.name.starts_with("_")
        //UNIQUE INDEX [A,B]
        && table.indices.iter().any(|i| {
            i.columns.len() == 2
                && is_a(&i.columns[0])
                && is_b(&i.columns[1])
                && i.tpe == IndexType::Unique
        })
        //INDEX [B]
        && table
            .indices
            .iter()
            .any(|i| i.columns.len() == 1 && is_b(&i.columns[0]) && i.tpe == IndexType::Normal)
        // 2 FKs
        && table.foreign_keys.len() == 2
        // Lexicographically lower model referenced by A
        && if table.foreign_keys[0].referenced_table <= table.foreign_keys[1].referenced_table {
            is_a(&table.foreign_keys[0].columns[0]) && is_b(&table.foreign_keys[1].columns[0])
        } else {
            is_b(&table.foreign_keys[0].columns[0]) && is_a(&table.foreign_keys[1].columns[0])
        }
}

pub(crate) fn is_foreign_key_column(table: &Table, column: &Column) -> bool {
    table
        .foreign_keys
        .iter()
        .find(|fk| fk.columns.contains(&column.name))
        .is_some()
}

//calculators

pub fn calculate_many_to_many_field(
    foreign_key: &ForeignKey,
    relation_name: String,
    is_self_relation: bool,
) -> Field {
    let field_type = FieldType::Relation(RelationInfo {
        name: relation_name,
        to: foreign_key.referenced_table.clone(),
        to_fields: foreign_key.referenced_columns.clone(),
        on_delete: OnDeleteStrategy::None,
    });

    let basename = foreign_key.referenced_table.camel_case();

    let name = match is_self_relation {
        true => format!("{}_{}", basename, foreign_key.columns[0]),
        false => basename,
    };

    Field {
        name,
        arity: FieldArity::List,
        field_type,
        database_names: Vec::new(),
        default_value: None,
        is_unique: false,
        is_id: false,
        documentation: None,
        is_generated: false,
        is_updated_at: false,
        data_source_fields: vec![],
        is_commented_out: false,
    }
}

pub(crate) fn calculate_index(index: &Index) -> IndexDefinition {
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
    index_definition
}

pub(crate) fn calculate_compound_index(index: &Index, name: String) -> IndexDefinition {
    debug!("Handling compound index  {:?}", name);
    IndexDefinition {
        name: Some(index.name.clone()),
        fields: vec![name],
        tpe: datamodel::dml::IndexType::Normal,
    }
}

pub(crate) fn calculate_scalar_field(schema: &SqlSchema, table: &Table, column: &Column) -> Field {
    debug!("Handling column {:?}", column);
    let field_type = calculate_field_type(&schema, &column, &table);
    let (is_commented_out, documentation) = match field_type {
        FieldType::Unsupported(_) => (
            true,
            Some("This type is currently not supported.".to_string()),
        ),
        _ => (false, None),
    };

    let arity = match column.tpe.arity {
        _ if column.auto_increment && field_type == FieldType::Base(ScalarType::Int, None) => {
            FieldArity::Required
        }
        ColumnArity::Required => FieldArity::Required,
        ColumnArity::Nullable => FieldArity::Optional,
        ColumnArity::List => FieldArity::List,
    };

    let is_id = is_id(&column, &table);
    let default_value = calculate_default(table, &column, &arity);
    let is_unique = table.is_column_unique(&column.name) && !is_id;

    Field {
        name: column.name.clone(),
        arity,
        field_type,
        database_names: vec![],
        default_value,
        is_unique,
        is_id,
        documentation,
        is_generated: false,
        is_updated_at: false,
        data_source_fields: vec![],
        is_commented_out,
    }
}

pub(crate) fn calculate_relation_field(
    schema: &SqlSchema,
    table: &Table,
    foreign_key: &ForeignKey,
    foreign_keys: &Vec<ForeignKey>,
) -> Field {
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

    let arity = match !columns.iter().any(|c| c.is_required()) {
        true => FieldArity::Optional,
        false => FieldArity::Required,
    };

    let more_then_one_compound_to_same_table = || {
        foreign_keys
            .iter()
            .filter(|fk| {
                fk.referenced_table == foreign_key.referenced_table && fk.columns.len() > 1
            })
            .count()
            > 1
    };

    let (name, database_name) = match columns.len() {
        1 => (columns[0].name.clone(), vec![]),
        _ if more_then_one_compound_to_same_table() => (
            format!(
                "{}_{}",
                foreign_key.referenced_table.clone().camel_case(),
                columns[0].name.clone()
            ),
            columns.iter().map(|c| c.name.clone()).collect(),
        ),
        _ => (
            foreign_key.referenced_table.clone().camel_case(),
            columns.iter().map(|c| c.name.clone()).collect(),
        ),
    };

    let is_id = is_relation_and_id(columns, &table);

    Field {
        name,
        arity,
        field_type,
        database_names: database_name,
        default_value: None,
        is_unique: false,
        is_id,
        documentation: None,
        is_generated: false,
        is_updated_at: false,
        data_source_fields: vec![],
        is_commented_out: false,
    }
}

pub(crate) fn calculate_backrelation_field(
    schema: &SqlSchema,
    model: &Model,
    other_model: &Model,
    relation_field: &Field,
    relation_info: &RelationInfo,
) -> Field {
    let table = schema.table_bang(&model.name);
    let fk = table.foreign_key_for_column(&relation_field.name);
    let on_delete = match fk {
        // TODO: bring `onDelete` back once `prisma migrate` is a thing
        //        Some(fk) if fk.on_delete_action == ForeignKeyAction::Cascade => OnDeleteStrategy::Cascade,
        _ => OnDeleteStrategy::None,
    };
    let field_type = FieldType::Relation(RelationInfo {
        name: relation_info.name.clone(),
        to: model.name.clone(),
        to_fields: vec![],
        on_delete,
    });

    let other_is_unique = || match &relation_field.database_names.len() {
        0 => table.is_column_unique(&relation_field.name),
        1 => {
            let column_name = relation_field.database_names.first().unwrap();
            table.is_column_unique(column_name)
        }
        _ => table
            .indices
            .iter()
            .any(|i| i.columns == relation_field.database_names && i.tpe == IndexType::Unique),
    };

    let arity = match relation_field.arity {
        FieldArity::Required | FieldArity::Optional if other_is_unique() => FieldArity::Optional,
        FieldArity::Required | FieldArity::Optional => FieldArity::List,
        FieldArity::List => FieldArity::Optional,
    };

    //if the backrelation name would be duplicate, probably due to being a selfrelation
    let name = if model.name == other_model.name && relation_field.name == model.name.camel_case() {
        format!("other_{}", model.name.clone().camel_case())
    } else {
        model.name.clone().camel_case()
    };

    Field {
        name,
        arity,
        field_type,
        database_names: vec![],
        default_value: None,
        is_unique: false,
        is_id: false,
        documentation: None,
        is_generated: false,
        is_updated_at: false,
        data_source_fields: vec![],
        is_commented_out: false,
    }
}

pub(crate) fn calculate_default(
    table: &Table,
    column: &Column,
    arity: &FieldArity,
) -> Option<DMLDef> {
    match (&column.default, &column.tpe.family) {
        (_, _) if *arity == FieldArity::List => None,
        (None, _) if column.auto_increment => Some(DMLDef::Expression(VG::new_autoincrement())),
        (Some(SQLDef::DBGENERATED(_)), _) => Some(DMLDef::Expression(VG::new_dbgenerated())),
        (Some(SQLDef::SEQUENCE(_)), _) => Some(DMLDef::Expression(VG::new_autoincrement())),
        (Some(SQLDef::VALUE(val)), ColumnTypeFamily::Boolean) => match parse_int(val) {
            Some(x) => Some(DMLDef::Single(SV::Boolean(x != 0))),
            None => parse_bool(val).map(|b| DMLDef::Single(SV::Boolean(b))),
        },
        (Some(SQLDef::VALUE(val)), ColumnTypeFamily::Int) => match column.auto_increment {
            true => Some(DMLDef::Expression(VG::new_autoincrement())),
            _ if is_sequence(column, table) => Some(DMLDef::Expression(VG::new_autoincrement())),
            false => parse_int(val).map(|x| DMLDef::Single(SV::Int(x))),
        },
        (Some(SQLDef::VALUE(val)), ColumnTypeFamily::Float) => {
            parse_float(val).map(|x| DMLDef::Single(SV::Float(x)))
        }
        (Some(SQLDef::VALUE(val)), ColumnTypeFamily::String) => {
            Some(DMLDef::Single(SV::String(val.into())))
        }
        (Some(SQLDef::NOW), ColumnTypeFamily::DateTime) => Some(DMLDef::Expression(VG::new_now())),
        (Some(SQLDef::VALUE(_)), ColumnTypeFamily::DateTime) => {
            Some(DMLDef::Expression(VG::new_dbgenerated()))
        } //todo parse datetime value
        (Some(SQLDef::VALUE(val)), ColumnTypeFamily::Enum(_)) => {
            Some(DMLDef::Single(SV::ConstantLiteral(val.into())))
        }
        (_, _) => None,
    }
}

pub(crate) fn is_id(column: &Column, table: &Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| pk.is_single_primary_key(&column.name))
        .unwrap_or(false)
}

pub(crate) fn is_relation_and_id(columns: Vec<&Column>, table: &Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| {
            columns_match(
                &pk.columns,
                &columns
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<Vec<String>>(),
            )
        })
        .unwrap_or(false)
}

pub(crate) fn is_part_of_id(column: &Column, table: &Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| pk.columns.contains(&column.name))
        .unwrap_or(false)
}

pub(crate) fn is_sequence(column: &Column, table: &Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| pk.is_single_primary_key(&column.name) && pk.sequence.is_some())
        .unwrap_or(false)
}

pub(crate) fn calculate_relation_name(
    schema: &SqlSchema,
    fk: &ForeignKey,
    table: &Table,
) -> String {
    //this is not called for prisma many to many relations. for them the name is just the name of the join table.
    let referenced_model = &fk.referenced_table;
    let model_with_fk = &table.name;
    let fk_column_name = fk.columns.join("_");

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

pub(crate) fn calculate_field_type(
    schema: &SqlSchema,
    column: &Column,
    table: &Table,
) -> FieldType {
    debug!("Calculating field type for '{}'", column.name);
    // Look for a foreign key referencing this column
    match table
        .foreign_keys
        .iter()
        .find(|fk| fk.columns.contains(&column.name))
    {
        Some(fk) if !is_id(column, table) && !is_part_of_id(column, table) => {
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
            match &column.tpe.family {
                ColumnTypeFamily::Boolean => FieldType::Base(ScalarType::Boolean, None),
                ColumnTypeFamily::DateTime => FieldType::Base(ScalarType::DateTime, None),
                ColumnTypeFamily::Float => FieldType::Base(ScalarType::Float, None),
                ColumnTypeFamily::Int => FieldType::Base(ScalarType::Int, None),
                ColumnTypeFamily::String => FieldType::Base(ScalarType::String, None),
                ColumnTypeFamily::Enum(name) => FieldType::Enum(name.clone()),
                ColumnTypeFamily::Uuid => FieldType::Base(ScalarType::String, None),
                ColumnTypeFamily::Json => FieldType::Base(ScalarType::String, None),
                x => FieldType::Unsupported(x.to_string()),
            }
        }
    }
}

// misc

pub fn deduplicate_names_of_fields_to_be_added(fields_to_be_added: &mut Vec<(String, Field)>) {
    let mut duplicated_relation_fields = Vec::new();
    fields_to_be_added
        .iter()
        .enumerate()
        .for_each(|(index, (model, field))| {
            let is_duplicated = fields_to_be_added
                .iter()
                .filter(|(other_model, other_field)| {
                    model == other_model && field.name == other_field.name
                })
                .count()
                > 1;

            if is_duplicated {
                duplicated_relation_fields.push(index);
            }
        });

    duplicated_relation_fields.iter().for_each(|index| {
        let (_, ref mut field) = fields_to_be_added.get_mut(*index).unwrap();
        field.name = match &field.field_type {
            FieldType::Relation(RelationInfo { name, .. }) => format!("{}_{}", field.name, &name),
            _ => field.name.clone(),
        };
    });
}

static RE_NUM: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'?(\d+)'?$").expect("compile regex"));

fn parse_int(value: &str) -> Option<i32> {
    debug!("Parsing int '{}'", value);
    let rslt = RE_NUM.captures(value);
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

static RE_FLOAT: Lazy<Regex> = Lazy::new(|| Regex::new(r"^'?([^']+)'?$").expect("compile regex"));

fn parse_float(value: &str) -> Option<f64> {
    debug!("Parsing float '{}'", value);
    let rslt = RE_FLOAT.captures(value);
    if rslt.is_none() {
        debug!("Couldn't parse float");
        return None;
    }

    let captures = rslt.expect("get captures");
    let num_str = captures.get(1).expect("get capture").as_str();
    let num_rslt = num_str.parse::<f64>();
    match num_rslt {
        Ok(num) => Some(num),
        Err(_) => {
            debug!("Couldn't parse float '{}'", num_str);
            None
        }
    }
}

/// Returns whether the elements of the two slices match, regardless of ordering.
pub fn columns_match(a_cols: &[String], b_cols: &[String]) -> bool {
    a_cols.len() == b_cols.len()
        && a_cols
            .iter()
            .all(|a_col| b_cols.iter().any(|b_col| a_col == b_col))
}

use datamodel::common::names::NameNormalizer;
use datamodel::{
    DefaultValue, Field, FieldArity, FieldType, IndexDefinition, Model, OnDeleteStrategy, RelationInfo, ScalarType,
    ScalarValue, ValueGenerator,
};
use log::debug;
use once_cell::sync::Lazy;
use regex::Regex;
use sql_schema_describer::{Column, ColumnArity, ColumnTypeFamily, ForeignKey, Index, IndexType, SqlSchema, Table};

//checks

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
        && table.indices.len() >= 1
        && table.indices.last().unwrap().columns.len() == 2
        && table.indices.last().unwrap().tpe == IndexType::Unique
}

pub(crate) fn is_foreign_key_column(table: &Table, column: &Column) -> bool {
    table
        .foreign_keys
        .iter()
        .find(|fk| fk.columns.contains(&column.name))
        .is_some()
}

//calculators

pub fn calculate_many_to_many_field(foreign_key: &ForeignKey, relation_name: String, is_self_relation: bool) -> Field {
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

pub(crate) fn calculate_scalar_field(schema: &&SqlSchema, table: &&Table, column: &&Column) -> Field {
    debug!("Handling column {:?}", column);
    let field_type = calculate_field_type(&schema, &column, &table);
    let arity = match column.tpe.arity {
        _ if column.auto_increment && field_type == FieldType::Base(ScalarType::Int) => FieldArity::Required,
        ColumnArity::Required => FieldArity::Required,
        ColumnArity::Nullable => FieldArity::Optional,
        ColumnArity::List => FieldArity::List,
    };

    let is_id = is_id(&column, &table);
    let default_value = calculate_default(&column, &arity);
    let is_unique = table.is_column_unique(&column.name) && !is_id;

    Field {
        name: column.name.clone(),
        arity,
        field_type,
        database_names: vec![],
        default_value,
        is_unique,
        is_id,
        documentation: None,
        is_generated: false,
        is_updated_at: false,
        data_source_fields: vec![],
    }
}

pub(crate) fn calculate_relation_field(schema: &SqlSchema, table: &Table, foreign_key: &ForeignKey) -> Field {
    debug!("Handling compound foreign key  {:?}", foreign_key);

    //todo this ignores relations on id fields of length 1, the problem persists for compound id fields
    if table.primary_key.is_some()
        && table.primary_key.as_ref().unwrap().columns == foreign_key.columns
        && foreign_key.columns.len() == 1
    {
        calculate_scalar_field(
            &schema,
            &table,
            &table.columns.iter().find(|c| c.name == foreign_key.columns[0]).unwrap(),
        )
    } else {
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

        let (name, database_name) = match columns.len() {
            1 => (columns[0].name.clone(), vec![]),
            _ => (
                foreign_key.referenced_table.clone().camel_case(),
                columns.iter().map(|c| c.name.clone()).collect(),
            ),
        };

        Field {
            name,
            arity,
            field_type,
            database_names: database_name,
            default_value: None,
            is_unique: false,
            is_id: false,
            documentation: None,
            is_generated: false,
            is_updated_at: false,
            data_source_fields: vec![],
        }
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
        to_fields: vec![relation_field.name.clone()],
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
    }
}

pub(crate) fn calculate_default(column: &Column, arity: &FieldArity) -> Option<DefaultValue> {
    match (&column.default, &column.tpe.family) {
        (_, _) if *arity == FieldArity::List => None,
        (Some(d), ColumnTypeFamily::Boolean) => match parse_int(d) {
            Some(x) => Some(DefaultValue::Single(ScalarValue::Boolean(x != 0))),
            None => parse_bool(d).map(|b| DefaultValue::Single(ScalarValue::Boolean(b))),
        },
        (Some(d), ColumnTypeFamily::Int) => match column.auto_increment {
            true => Some(DefaultValue::Expression(ValueGenerator::new_autoincrement())),
            false => parse_int(d).map(|x| DefaultValue::Single(ScalarValue::Int(x))),
        },
        (Some(d), ColumnTypeFamily::Float) => parse_float(d).map(|x| DefaultValue::Single(ScalarValue::Float(x))),
        (Some(d), ColumnTypeFamily::String) => Some(DefaultValue::Single(ScalarValue::String(d.to_string()))),
        (Some(_), ColumnTypeFamily::DateTime) => None, //todo
        (None, _) if column.auto_increment => Some(DefaultValue::Expression(ValueGenerator::new_autoincrement())),
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

pub(crate) fn calculate_relation_name(schema: &SqlSchema, fk: &ForeignKey, table: &Table) -> String {
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

pub(crate) fn calculate_field_type(schema: &SqlSchema, column: &Column, table: &Table) -> FieldType {
    debug!("Calculating field type for '{}'", column.name);
    // Look for a foreign key referencing this column
    match table.foreign_keys.iter().find(|fk| fk.columns.contains(&column.name)) {
        Some(fk) if !is_id(column, table) => {
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
                ColumnTypeFamily::Boolean => FieldType::Base(ScalarType::Boolean),
                ColumnTypeFamily::DateTime => FieldType::Base(ScalarType::DateTime),
                ColumnTypeFamily::Float => FieldType::Base(ScalarType::Float),
                ColumnTypeFamily::Int => FieldType::Base(ScalarType::Int),
                ColumnTypeFamily::String => FieldType::Base(ScalarType::String),
                ColumnTypeFamily::Enum(name) => FieldType::Enum(name.clone()),
                // XXX: We made a conscious decision to punt on mapping of ColumnTypeFamily
                // variants that don't yet have corresponding PrismaType variants
                _ => FieldType::Base(ScalarType::String),
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
                .filter(|(other_model, other_field)| model == other_model && field.name == other_field.name)
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

fn parse_float(value: &str) -> Option<f32> {
    debug!("Parsing float '{}'", value);
    let rslt = RE_FLOAT.captures(value);
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

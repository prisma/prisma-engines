use crate::calculate_datamodel::CalculateDatamodelContext as Context;
use datamodel::dml;
use sql_schema_describer::{self as sql, postgres::PostgresSchemaExt};

pub(crate) fn calculate_default(
    table: &sql::Table,
    column: &sql::Column,
    ctx: &mut Context,
) -> Option<dml::DefaultValue> {
    match (column.default.as_ref().map(|d| d.kind()), &column.tpe.family) {
        (Some(sql::DefaultKind::Sequence(name)), _) if ctx.is_cockroach() => {
            use prisma_value::PrismaValue;

            let connector_data: &PostgresSchemaExt = ctx.schema.downcast_connector_data().unwrap_or_default();
            let sequence_idx = connector_data
                .sequences
                .binary_search_by_key(&name, |s| &s.name)
                .unwrap();
            let sequence = &connector_data.sequences[sequence_idx];

            let mut args = Vec::new();

            if sequence.min_value != 1 {
                args.push((Some("minValue".to_owned()), PrismaValue::Int(sequence.min_value)));
            }

            if sequence.max_value != i64::MAX {
                args.push((Some("maxValue".to_owned()), PrismaValue::Int(sequence.max_value)));
            }

            if sequence.cache_size != 1 {
                args.push((Some("cache".to_owned()), PrismaValue::Int(sequence.cache_size)));
            }

            if sequence.increment_by != 1 {
                args.push((Some("increment".to_owned()), PrismaValue::Int(sequence.increment_by)));
            }

            if sequence.start_value != 1 {
                args.push((Some("start".to_owned()), PrismaValue::Int(sequence.start_value)));
            }

            Some(dml::DefaultValue::new_expression(dml::ValueGenerator::new_sequence(
                args,
            )))
        }
        (_, sql::ColumnTypeFamily::Int) if column.auto_increment => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (_, sql::ColumnTypeFamily::BigInt) if column.auto_increment => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (_, sql::ColumnTypeFamily::Int) if is_sequence(column, table) => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (_, sql::ColumnTypeFamily::BigInt) if is_sequence(column, table) => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (Some(sql::DefaultKind::Sequence(_)), _) => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (Some(sql::DefaultKind::Now), sql::ColumnTypeFamily::DateTime) => Some(set_default(
            dml::DefaultValue::new_expression(dml::ValueGenerator::new_now()),
            column,
        )),
        (Some(sql::DefaultKind::DbGenerated(default_string)), _) => Some(set_default(
            dml::DefaultValue::new_expression(dml::ValueGenerator::new_dbgenerated(default_string.clone())),
            column,
        )),
        (Some(sql::DefaultKind::Value(val)), _) => {
            Some(set_default(dml::DefaultValue::new_single(val.clone()), column))
        }
        (Some(sql::DefaultKind::UniqueRowid), _) => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        _ => None,
    }
}

fn set_default(mut default: dml::DefaultValue, column: &sql::Column) -> dml::DefaultValue {
    let db_name = column.default.as_ref().and_then(|df| df.constraint_name());

    if let Some(name) = db_name {
        default.set_db_name(name);
    }

    default
}

fn is_sequence(column: &sql::Column, table: &sql::Table) -> bool {
    table
        .primary_key
        .as_ref()
        .map(|pk| pk.is_single_primary_key(&column.name) && matches!(&column.default, Some(d) if d.is_sequence()))
        .unwrap_or(false)
}

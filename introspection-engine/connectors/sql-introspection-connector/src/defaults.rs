use crate::{calculate_datamodel::CalculateDatamodelContext as Context, SqlFamilyTrait};
use introspection_connector::Version;
use psl::{
    builtin_connectors::{MySqlType, PostgresType},
    dml,
    parser_database::walkers,
};
use sql_schema_describer::{self as sql, postgres::PostgresSchemaExt};

pub(crate) fn calculate_default(
    column: sql::ColumnWalker<'_>,
    existing_field: Option<walkers::ScalarFieldWalker<'_>>,
    ctx: &mut Context<'_>,
) -> Option<dml::DefaultValue> {
    match (column.default().map(|d| d.kind()), &column.column_type_family()) {
        (Some(sql::DefaultKind::Sequence(name)), _) if ctx.is_cockroach() => {
            use dml::PrismaValue;

            let connector_data: &PostgresSchemaExt = ctx.schema.downcast_connector_data();
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
        (_, sql::ColumnTypeFamily::Int) if column.is_autoincrement() => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (_, sql::ColumnTypeFamily::BigInt) if column.is_autoincrement() => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (_, sql::ColumnTypeFamily::Int) if is_sequence(column) => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),
        (_, sql::ColumnTypeFamily::BigInt) if is_sequence(column) => Some(dml::DefaultValue::new_expression(
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
            dml::DefaultValue::new_expression(dml::ValueGenerator::new_dbgenerated(
                default_string.as_ref().unwrap().clone(),
            )),
            column,
        )),
        (Some(sql::DefaultKind::Value(dml::PrismaValue::Enum(variant))), sql::ColumnTypeFamily::Enum(enum_id)) => {
            let variant = ctx
                .schema
                .walk(*enum_id)
                .variants()
                .find(|v| v.name() == variant)
                .unwrap();
            let variant_name = ctx.enum_variant_name(variant.id).prisma_name();
            Some(set_default(
                dml::DefaultValue::new_single(dml::PrismaValue::Enum(variant_name.into_owned())),
                column,
            ))
        }
        (Some(sql::DefaultKind::Value(val)), _) => {
            Some(set_default(dml::DefaultValue::new_single(val.clone()), column))
        }
        (Some(sql::DefaultKind::UniqueRowid), _) => Some(dml::DefaultValue::new_expression(
            dml::ValueGenerator::new_autoincrement(),
        )),

        // Prisma-level defaults.
        (None, sql::ColumnTypeFamily::String) => match existing_field.and_then(|f| f.default_value()) {
            Some(value) if value.is_cuid() => Some(default_cuid()),
            Some(value) if value.is_uuid() => Some(default_uuid()),
            None if matches!(ctx.version, Version::Prisma1 | Version::Prisma11) => maybe_prisma1_default(column, ctx),
            _ => None,
        },

        _ => None,
    }
}

fn set_default(mut default: dml::DefaultValue, column: sql::ColumnWalker<'_>) -> dml::DefaultValue {
    let db_name = column.default().and_then(|df| df.constraint_name());

    if let Some(name) = db_name {
        default.set_db_name(name);
    }

    default
}

fn is_sequence(column: sql::ColumnWalker<'_>) -> bool {
    column.is_single_primary_key() && matches!(&column.default(), Some(d) if d.is_sequence())
}

fn maybe_prisma1_default(column: sql::ColumnWalker<'_>, ctx: &mut Context<'_>) -> Option<dml::DefaultValue> {
    let model_and_field = || crate::warnings::ModelAndField {
        model: ctx.table_prisma_name(column.table().id).prisma_name().into_owned(),
        field: ctx.column_prisma_name(column.id).prisma_name().into_owned(),
    };

    if ctx.sql_family().is_postgres() {
        let native_type: &PostgresType = column.column_type().native_type.as_ref()?.downcast_ref();

        if native_type == &PostgresType::VarChar(Some(25)) {
            ctx.prisma_1_cuid_defaults.push(model_and_field());
            return Some(default_cuid());
        } else if native_type == &PostgresType::VarChar(Some(36)) {
            ctx.prisma_1_uuid_defaults.push(model_and_field());
            return Some(default_uuid());
        }
    } else if ctx.sql_family().is_mysql() {
        let native_type: &MySqlType = column.column_type().native_type.as_ref()?.downcast_ref();

        if native_type == &MySqlType::Char(25) {
            ctx.prisma_1_cuid_defaults.push(model_and_field());
            return Some(default_cuid());
        } else if native_type == &MySqlType::Char(36) {
            ctx.prisma_1_uuid_defaults.push(model_and_field());
            return Some(default_uuid());
        }
    };

    None
}

fn default_cuid() -> dml::DefaultValue {
    dml::DefaultValue::new_expression(dml::ValueGenerator::new_cuid())
}

fn default_uuid() -> dml::DefaultValue {
    dml::DefaultValue::new_expression(dml::ValueGenerator::new_uuid())
}

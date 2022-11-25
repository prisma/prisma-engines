use crate::{calculate_datamodel::CalculateDatamodelContext as Context, SqlFamilyTrait};
use datamodel_renderer::datamodel as renderer;
use introspection_connector::Version;
use psl::{
    builtin_connectors::{MySqlType, PostgresType},
    datamodel_connector::constraint_names::ConstraintNames,
    dml,
    parser_database::walkers,
};
use sql_schema_describer::{self as sql, postgres::PostgresSchemaExt};

pub(crate) fn render_default<'a>(
    column: sql::ColumnWalker<'a>,
    existing_field: Option<walkers::ScalarFieldWalker<'a>>,
    ctx: &mut Context<'a>,
) -> Option<renderer::DefaultValue<'a>> {
    use datamodel_renderer::value::{Constant, Function, Text, Value};

    let mut result = match (column.default().map(|d| d.kind()), column.column_type_family()) {
        (Some(sql::DefaultKind::Sequence(name)), _) if ctx.is_cockroach() => {
            let connector_data: &PostgresSchemaExt = ctx.schema.downcast_connector_data();

            let sequence_idx = connector_data
                .sequences
                .binary_search_by_key(&name, |s| &s.name)
                .unwrap();

            let sequence = &connector_data.sequences[sequence_idx];

            let mut fun = Function::new("sequence");

            if sequence.min_value != 1 {
                fun.push_param(("minValue", Constant::from(sequence.min_value)));
            }

            if sequence.max_value != i64::MAX {
                fun.push_param(("maxValue", Constant::from(sequence.max_value)));
            }

            if sequence.cache_size != 1 {
                fun.push_param(("cache", Constant::from(sequence.cache_size)));
            }

            if sequence.increment_by != 1 {
                fun.push_param(("increment", Constant::from(sequence.increment_by)));
            }

            if sequence.start_value != 1 {
                fun.push_param(("start", Constant::from(sequence.start_value)));
            }

            Some(renderer::DefaultValue::function(fun))
        }
        (_, sql::ColumnTypeFamily::Int | sql::ColumnTypeFamily::BigInt) if column.is_autoincrement() => {
            Some(renderer::DefaultValue::function(Function::new("autoincrement")))
        }
        (_, sql::ColumnTypeFamily::Int | sql::ColumnTypeFamily::BigInt) if is_sequence(column) => {
            Some(renderer::DefaultValue::function(Function::new("autoincrement")))
        }
        (Some(sql::DefaultKind::Sequence(_)), _) => {
            Some(renderer::DefaultValue::function(Function::new("autoincrement")))
        }
        (Some(sql::DefaultKind::UniqueRowid), _) => {
            Some(renderer::DefaultValue::function(Function::new("autoincrement")))
        }
        (Some(sql::DefaultKind::Now), sql::ColumnTypeFamily::DateTime) => {
            Some(renderer::DefaultValue::function(Function::new("now")))
        }
        (Some(sql::DefaultKind::DbGenerated(default_string)), _) => {
            let mut fun = Function::new("dbgenerated");

            if let Some(param) = default_string.as_ref().filter(|s| !s.trim_matches('\0').is_empty()) {
                fun.push_param(Value::from(Text::new(param)));
            }

            Some(renderer::DefaultValue::function(fun))
        }
        (Some(sql::DefaultKind::Value(dml::PrismaValue::Enum(variant))), sql::ColumnTypeFamily::Enum(enum_id)) => {
            let variant = ctx
                .schema
                .walk(*enum_id)
                .variants()
                .find(|v| v.name() == variant)
                .unwrap();

            let variant_name = ctx.enum_variant_name(variant.id).prisma_name();
            Some(renderer::DefaultValue::constant(variant_name))
        }
        (Some(sql::DefaultKind::Value(dml::PrismaValue::String(val))), _) => Some(renderer::DefaultValue::text(val)),
        (Some(sql::DefaultKind::Value(dml::PrismaValue::List(val))), _) => {
            let vals = val
                .iter()
                .map(|val| match val {
                    dml::PrismaValue::String(v) => Value::from(Text::new(v)),
                    dml::PrismaValue::Boolean(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::Enum(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::Int(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::Uuid(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::List(_) => unreachable!("Lists of lists are not supported in defaults."),
                    dml::PrismaValue::Json(v) => Value::from(Text::new(v)),
                    dml::PrismaValue::Xml(v) => Value::from(Text::new(v)),
                    dml::PrismaValue::Object(_) => unreachable!("Objects are not supported in defaults."),
                    dml::PrismaValue::Null => Value::from(Constant::from("null")),
                    dml::PrismaValue::DateTime(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::Float(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::BigInt(v) => Value::from(Constant::from(v)),
                    dml::PrismaValue::Bytes(v) => Value::from(v.clone()),
                })
                .collect();

            Some(renderer::DefaultValue::array(vals))
        }
        (Some(sql::DefaultKind::Value(val)), _) => Some(renderer::DefaultValue::constant(val)),

        // Prisma-level defaults.
        (None, sql::ColumnTypeFamily::String) => match existing_field.and_then(|f| f.default_value()) {
            Some(value) if value.is_cuid() => Some(renderer::DefaultValue::function(Function::new("cuid"))),
            Some(value) if value.is_uuid() => Some(renderer::DefaultValue::function(Function::new("uuid"))),
            None if matches!(ctx.version, Version::Prisma1 | Version::Prisma11) => maybe_prisma1_default(column, ctx),
            _ => None,
        },

        _ => None,
    };

    if let Some(res) = result.as_mut() {
        let default_default_value =
            ConstraintNames::default_name(column.table().name(), column.name(), ctx.active_connector());

        match column.default().and_then(|def| def.constraint_name()) {
            Some(map) if map != default_default_value => {
                res.map(map);
            }
            _ => (),
        }
    }

    result
}

fn is_sequence(column: sql::ColumnWalker<'_>) -> bool {
    column.is_single_primary_key() && matches!(&column.default(), Some(d) if d.is_sequence())
}

fn maybe_prisma1_default<'a>(
    column: sql::ColumnWalker<'a>,
    ctx: &mut Context<'a>,
) -> Option<renderer::DefaultValue<'a>> {
    use datamodel_renderer::value::Function;

    let model_and_field = || crate::warnings::ModelAndField {
        model: ctx.table_prisma_name(column.table().id).prisma_name().into_owned(),
        field: ctx.column_prisma_name(column.id).prisma_name().into_owned(),
    };

    if ctx.sql_family().is_postgres() {
        let native_type: &PostgresType = column.column_type().native_type.as_ref()?.downcast_ref();

        if native_type == &PostgresType::VarChar(Some(25)) {
            ctx.prisma_1_cuid_defaults.push(model_and_field());

            return Some(renderer::DefaultValue::function(Function::new("cuid")));
        } else if native_type == &PostgresType::VarChar(Some(36)) {
            ctx.prisma_1_uuid_defaults.push(model_and_field());

            return Some(renderer::DefaultValue::function(Function::new("uuid")));
        }
    } else if ctx.sql_family().is_mysql() {
        let native_type: &MySqlType = column.column_type().native_type.as_ref()?.downcast_ref();

        if native_type == &MySqlType::Char(25) {
            ctx.prisma_1_cuid_defaults.push(model_and_field());

            return Some(renderer::DefaultValue::function(Function::new("cuid")));
        } else if native_type == &MySqlType::Char(36) {
            ctx.prisma_1_uuid_defaults.push(model_and_field());

            return Some(renderer::DefaultValue::function(Function::new("uuid")));
        }
    }

    None
}

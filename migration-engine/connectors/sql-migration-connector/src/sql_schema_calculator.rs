mod sql_schema_calculator_flavour;

pub(super) use sql_schema_calculator_flavour::SqlSchemaCalculatorFlavour;

use crate::{flavour::SqlFlavour, SqlDatabaseSchema};
use datamodel::{
    datamodel_connector::{walker_ext_traits::*, ReferentialAction, ScalarType},
    dml::{prisma_value, PrismaValue},
    parser_database::{ast, walkers::ScalarFieldWalker, IndexType, ScalarFieldType, SortOrder},
    ValidatedSchema,
};
use sql_schema_describer as sql;
use std::collections::HashMap;

pub(crate) fn calculate_sql_schema(datamodel: &ValidatedSchema, flavour: &dyn SqlFlavour) -> SqlDatabaseSchema {
    let mut schema = SqlDatabaseSchema::default();

    schema.describer_schema.enums = flavour.calculate_enums(datamodel);

    let mut context = Context {
        datamodel,
        schema: &mut schema,
        flavour,
        model_id_to_table_id: HashMap::with_capacity(datamodel.db.models_count()),
    };

    // Two types of tables: model tables and implicit M2M relation tables (a.k.a. join tables.).
    push_model_tables(&mut context);

    if context.datamodel.referential_integrity().uses_foreign_keys() {
        push_inline_relations(&mut context);
    }

    push_relation_tables(&mut context);
    flavour.push_connector_data(&mut context);

    schema
}

fn push_model_tables(ctx: &mut Context<'_>) {
    for model in ctx.datamodel.db.walk_models() {
        let table_id = ctx.schema.describer_schema.push_table(model.database_name().to_owned());
        ctx.model_id_to_table_id.insert(model.model_id(), table_id);
        let table = &mut ctx.schema.describer_schema[table_id];

        table.primary_key = model.primary_key().map(|pk| sql::PrimaryKey {
            columns: pk
                .scalar_field_attributes()
                .map(|field| sql::PrimaryKeyColumn {
                    name: field.as_index_field().database_name().to_owned(),
                    length: field.length(),
                    sort_order: field.sort_order().map(|so| match so {
                        SortOrder::Asc => sql::SQLSortOrder::Asc,
                        SortOrder::Desc => sql::SQLSortOrder::Desc,
                    }),
                })
                .collect(),
            constraint_name: pk
                .constraint_name(ctx.flavour.datamodel_connector())
                .map(|c| c.into_owned()),
        });

        table.indices = model
            .indexes()
            .map(|index| {
                let columns = index
                    .scalar_field_attributes()
                    .map(|sf| sql::IndexColumn {
                        name: sf.as_index_field().database_name().to_owned(),
                        sort_order: sf.sort_order().map(|s| match s {
                            SortOrder::Asc => sql::SQLSortOrder::Asc,
                            SortOrder::Desc => sql::SQLSortOrder::Desc,
                        }),
                        length: sf.length(),
                    })
                    .collect();

                let index_type = match index.index_type() {
                    IndexType::Unique => sql::IndexType::Unique,
                    IndexType::Normal => sql::IndexType::Normal,
                    IndexType::Fulltext => sql::IndexType::Fulltext,
                };

                sql::Index {
                    name: index.constraint_name(ctx.flavour.datamodel_connector()).into_owned(),
                    columns,
                    tpe: index_type,
                }
            })
            .collect();

        for field in model.scalar_fields() {
            push_column_for_scalar_field(field, table_id, ctx);
        }
    }
}

fn push_inline_relations(ctx: &mut Context<'_>) {
    for relation in ctx.datamodel.db.walk_relations().filter_map(|r| r.refine().as_inline()) {
        let relation_field = relation
            .forward_relation_field()
            .expect("Expecting a complete relation in sql_schmea_calculator");
        let referencing_model = ctx.model_id_to_table_id[&relation_field.model().model_id()];
        let referenced_model = ctx.model_id_to_table_id[&relation.referenced_model().model_id()];
        let on_delete_action = relation_field.explicit_on_delete().unwrap_or_else(|| {
            relation_field.default_on_delete_action(
                ctx.datamodel.configuration.referential_integrity().unwrap_or_default(),
                ctx.flavour.datamodel_connector(),
            )
        });
        let on_update_action = relation_field
            .explicit_on_update()
            .map(convert_referential_action)
            .unwrap_or_else(|| sql::ForeignKeyAction::Cascade);

        let fkid = ctx.schema.describer_schema.push_foreign_key(
            Some(relation.constraint_name(ctx.flavour.datamodel_connector()).into_owned()),
            [referencing_model, referenced_model],
            [convert_referential_action(on_delete_action), on_update_action],
        );

        let columns = relation_field
            .fields()
            .unwrap()
            .zip(relation_field.referenced_fields().unwrap());

        for (referencing, referenced) in columns {
            let column = [
                ctx.walk(referencing_model)
                    .column(referencing.database_name())
                    .unwrap()
                    .id,
                ctx.walk(referenced_model)
                    .column(referenced.database_name())
                    .unwrap()
                    .id,
            ];
            ctx.schema.describer_schema.push_foreign_key_column(fkid, column);
        }
    }
}

fn push_relation_tables(ctx: &mut Context<'_>) {
    let datamodel = ctx.datamodel;
    let flavour = ctx.flavour;
    let m2m_relations = datamodel
        .db
        .walk_relations()
        .filter_map(|relation| relation.refine().as_many_to_many());

    for m2m in m2m_relations {
        let table_name = format!("_{}", m2m.relation_name());
        let table_name = table_name
            .chars()
            .take(datamodel.configuration.max_identifier_length())
            .collect::<String>();
        let model_a = m2m.model_a();
        let model_a_table_id = ctx.model_id_to_table_id[&model_a.model_id()];
        let model_b = m2m.model_b();
        let model_b_table_id = ctx.model_id_to_table_id[&model_b.model_id()];
        let model_a_column = "A";
        let model_b_column = "B";
        let model_a_id = model_a.primary_key().unwrap().fields().next().unwrap();
        let model_b_id = model_b.primary_key().unwrap().fields().next().unwrap();

        let max_identifier_length = ctx.flavour.datamodel_connector().max_identifier_length();
        let fk_suffix = "_fkey";
        let max_table_name_len = max_identifier_length - fk_suffix.len() - 2;
        // We slightly diverge from the default naming conventions here, because we want to
        // completely exclude the possibility of collisions, since these names are not
        // configurable in implicit many-to-many relation tables.
        let model_a_fk_name = if table_name.len() > max_table_name_len {
            format!("{}_A{fk_suffix}", &table_name[0..max_table_name_len])
        } else {
            format!("{table_name}_A{fk_suffix}")
        };
        let model_b_fk_name = if table_name.len() >= max_table_name_len {
            format!("{}_B{fk_suffix}", &table_name[0..max_table_name_len])
        } else {
            format!("{table_name}_B{fk_suffix}")
        };

        let table_id = ctx.schema.describer_schema.push_table(table_name);
        let column_a_type = ctx
            .walk(model_a_table_id)
            .primary_key_columns()
            .next()
            .unwrap()
            .as_column()
            .column_type()
            .clone();
        let column_b_type = ctx
            .walk(model_b_table_id)
            .primary_key_columns()
            .next()
            .unwrap()
            .as_column()
            .column_type()
            .clone();

        let column_a_id = ctx.schema.describer_schema.push_column(
            table_id,
            sql::Column {
                name: model_a_column.into(),
                tpe: column_a_type,
                default: None,
                auto_increment: false,
            },
        );
        let column_b_id = ctx.schema.describer_schema.push_column(
            table_id,
            sql::Column {
                name: model_b_column.into(),
                tpe: column_b_type,
                default: None,
                auto_increment: false,
            },
        );

        {
            let mut table = &mut ctx.schema.describer_schema[table_id];
            table.indices = vec![
                sql::Index {
                    name: format!(
                        "{}_AB_unique",
                        table.name.chars().take(max_identifier_length - 10).collect::<String>()
                    ),
                    columns: vec![
                        sql::IndexColumn::new(model_a_column),
                        sql::IndexColumn::new(model_b_column),
                    ],
                    tpe: sql::IndexType::Unique,
                },
                sql::Index {
                    name: format!(
                        "{}_B_index",
                        table.name.chars().take(max_identifier_length - 8).collect::<String>()
                    ),
                    columns: vec![sql::IndexColumn::new(model_b_column)],
                    tpe: sql::IndexType::Normal,
                },
            ];
        }

        if ctx.datamodel.referential_integrity().uses_foreign_keys() {
            let fkid = ctx.schema.describer_schema.push_foreign_key(
                Some(model_a_fk_name),
                [table_id, ctx.model_id_to_table_id[&model_a.model_id()]],
                [flavour.m2m_foreign_key_action(model_a, model_b); 2],
            );

            ctx.schema.describer_schema.push_foreign_key_column(
                fkid,
                [
                    column_a_id,
                    ctx.schema
                        .describer_schema
                        .walk(model_a_table_id)
                        .column(model_a_id.database_name())
                        .unwrap()
                        .id,
                ],
            );

            let fkid = ctx.schema.describer_schema.push_foreign_key(
                Some(model_b_fk_name),
                [table_id, ctx.model_id_to_table_id[&model_b.model_id()]],
                [flavour.m2m_foreign_key_action(model_a, model_b); 2],
            );

            ctx.schema.describer_schema.push_foreign_key_column(
                fkid,
                [
                    column_b_id,
                    ctx.schema
                        .describer_schema
                        .walk(model_b_table_id)
                        .column(model_b_id.database_name())
                        .unwrap()
                        .id,
                ],
            );
        }
    }
}

fn push_column_for_scalar_field(field: ScalarFieldWalker<'_>, table_id: sql::TableId, ctx: &mut Context<'_>) {
    match field.scalar_field_type() {
        ScalarFieldType::Enum(enum_id) => push_column_for_model_enum_scalar_field(field, enum_id, table_id, ctx),
        ScalarFieldType::CompositeType(_) => {
            push_column_for_builtin_scalar_type(field, ScalarType::Json, table_id, ctx)
        }
        ScalarFieldType::BuiltInScalar(scalar_type) => {
            push_column_for_builtin_scalar_type(field, scalar_type, table_id, ctx)
        }
        ScalarFieldType::Unsupported(_) => push_column_for_model_unsupported_scalar_field(field, table_id, ctx),
    }
}

fn push_column_for_model_enum_scalar_field(
    field: ScalarFieldWalker<'_>,
    enum_id: ast::EnumId,
    table_id: sql::TableId,
    ctx: &mut Context<'_>,
) {
    let r#enum = ctx.datamodel.db.walk_enum(enum_id);
    let value_for_name = |name: &str| -> PrismaValue {
        match r#enum.values().find(|v| v.name() == name).map(|v| v.database_name()) {
            Some(v) => PrismaValue::Enum(v.to_owned()),
            None => panic!("Expected enum field default to reference existing value."),
        }
    };

    let default = field.default_value().and_then(|def| match def.value() {
        ast::Expression::ConstantValue(value_name, _) => {
            let def = sql::DefaultValue::value(value_for_name(value_name))
                .with_constraint_name(ctx.flavour.default_constraint_name(def));
            Some(def)
        }
        ast::Expression::Array(items, _) => {
            let mut values = Vec::with_capacity(items.len());

            for item in items {
                let (value_name, _) = item
                    .as_constant_value()
                    .expect("Non-constant value inside enum list default.");
                values.push(value_for_name(value_name));
            }

            let default_value = sql::DefaultValue::value(PrismaValue::List(values))
                .with_constraint_name(ctx.flavour.default_constraint_name(def));
            Some(default_value)
        }
        other => unwrap_dbgenerated(other).map(|value| {
            sql::DefaultValue::db_generated(value).with_constraint_name(ctx.flavour.default_constraint_name(def))
        }),
    });
    ctx.schema.describer_schema.push_column(
        table_id,
        sql::Column {
            name: field.database_name().to_owned(),
            tpe: ctx.flavour.enum_column_type(field, r#enum.database_name()),
            default,
            auto_increment: false,
        },
    );
}

fn push_column_for_model_unsupported_scalar_field(
    field: ScalarFieldWalker<'_>,
    table_id: sql::TableId,
    ctx: &mut Context<'_>,
) {
    ctx.schema.describer_schema.push_column(
        table_id,
        sql::Column {
            name: field.database_name().to_owned(),
            tpe: ctx.flavour.column_type_for_unsupported_type(
                field,
                field.ast_field().field_type.as_unsupported().unwrap().0.to_owned(),
            ),
            default: field.default_value().and_then(|def| {
                // This is validated as @default(dbgenerated("...")), we can unwrap.
                let dbgenerated_contents = unwrap_dbgenerated(def.value());
                if let Some(value) = dbgenerated_contents {
                    let default = sql::DefaultValue::db_generated(value)
                        .with_constraint_name(ctx.flavour.default_constraint_name(def));
                    Some(default)
                } else {
                    None
                }
            }),
            auto_increment: false,
        },
    );
}

fn push_column_for_builtin_scalar_type(
    field: ScalarFieldWalker<'_>,
    scalar_type: ScalarType,
    table_id: sql::TableId,
    ctx: &mut Context<'_>,
) {
    let connector = ctx.flavour.datamodel_connector();
    let family = match scalar_type {
        ScalarType::Int => sql::ColumnTypeFamily::Int,
        ScalarType::Float => sql::ColumnTypeFamily::Float,
        ScalarType::Boolean => sql::ColumnTypeFamily::Boolean,
        ScalarType::String => sql::ColumnTypeFamily::String,
        ScalarType::DateTime => sql::ColumnTypeFamily::DateTime,
        ScalarType::Json => sql::ColumnTypeFamily::Json,
        ScalarType::Bytes => sql::ColumnTypeFamily::Binary,
        ScalarType::Decimal => sql::ColumnTypeFamily::Decimal,
        ScalarType::BigInt => sql::ColumnTypeFamily::BigInt,
    };

    let native_type = field
        .native_type_instance(connector)
        .map(|instance| instance.serialized_native_type)
        .unwrap_or_else(|| connector.default_native_type_for_scalar_type(&scalar_type));

    let column_id = ctx.schema.describer_schema.push_column(
        table_id,
        sql::Column {
            name: field.database_name().to_owned(),
            default: None,
            tpe: sql::ColumnType {
                family,
                full_data_type: String::new(),
                arity: column_arity(field.ast_field().arity),
                native_type: Some(native_type),
            },
            auto_increment: field.is_autoincrement() || ctx.flavour.field_is_implicit_autoincrement_primary_key(field),
        },
    );

    if let Some(v) = field.default_value() {
        let default_value = {
            if v.is_dbgenerated() {
                unwrap_dbgenerated(v.value()).map(|v| sql::DefaultValue::new(sql::DefaultKind::DbGenerated(v)))
            } else if v.is_now() {
                Some(sql::DefaultValue::now())
            } else if v.is_autoincrement() {
                ctx.flavour.column_default_value_for_autoincrement()
            } else if v.is_sequence() {
                Some(sql::DefaultValue::new(sql::DefaultKind::Sequence(format!(
                    "prisma_sequence_{}_{}",
                    field.model().database_name(),
                    field.database_name()
                ))))
            } else {
                match v.value() {
                    ast::Expression::Function(_, _, _) => {
                        // prisma-generated
                        ctx.schema.prisma_level_defaults.push(column_id);
                        return;
                    }
                    constant => Some(sql::DefaultValue::new(sql::DefaultKind::Value(
                        constant_expression_to_sql_default(constant, scalar_type),
                    ))),
                }
            }
        };

        let (_, column) = &mut ctx.schema.describer_schema[column_id];
        column.default = default_value.map(|df| df.with_constraint_name(ctx.flavour.default_constraint_name(v)));
    }
}

fn constant_expression_to_sql_default(expr: &ast::Expression, scalar_type: ScalarType) -> PrismaValue {
    match expr {
        ast::Expression::NumericValue(val, _) => match scalar_type {
            ScalarType::Int => PrismaValue::Int(val.parse().unwrap()),
            ScalarType::BigInt => PrismaValue::BigInt(val.parse().unwrap()),
            ScalarType::Float | ScalarType::Decimal => PrismaValue::Float(val.parse().unwrap()),
            other => unreachable!("{:?}", other),
        },
        ast::Expression::StringValue(val, _) => match scalar_type {
            ScalarType::String => PrismaValue::String(val.clone()),
            ScalarType::DateTime => PrismaValue::DateTime(val.parse().unwrap()),
            ScalarType::Json => PrismaValue::Json(val.parse().unwrap()),
            ScalarType::Bytes => PrismaValue::Bytes(prisma_value::decode_bytes(val).unwrap()),
            ScalarType::Decimal => PrismaValue::Float(val.parse().unwrap()),
            other => unreachable!("{:?}", other),
        },

        ast::Expression::Array(items, _) => {
            let mut values: Vec<PrismaValue> = Vec::with_capacity(items.len());

            for item in items {
                values.push(constant_expression_to_sql_default(item, scalar_type));
            }

            PrismaValue::List(values)
        }

        // The only case where we have constant defaults in scalars is booleans.
        ast::Expression::ConstantValue(val, _) => PrismaValue::Boolean(val.parse().unwrap()),

        // Handled before this function is called.
        ast::Expression::Function(_, _, _) => unreachable!(),
    }
}

fn column_arity(arity: ast::FieldArity) -> sql::ColumnArity {
    match &arity {
        ast::FieldArity::Required => sql::ColumnArity::Required,
        ast::FieldArity::List => sql::ColumnArity::List,
        ast::FieldArity::Optional => sql::ColumnArity::Nullable,
    }
}

pub(crate) struct Context<'a> {
    datamodel: &'a ValidatedSchema,
    schema: &'a mut SqlDatabaseSchema,
    flavour: &'a dyn SqlFlavour,
    model_id_to_table_id: HashMap<ast::ModelId, sql::TableId>,
}

impl Context<'_> {
    fn walk<I>(&self, id: I) -> sql::Walker<'_, I> {
        self.schema.walk(id)
    }
}

fn convert_referential_action(action: ReferentialAction) -> sql::ForeignKeyAction {
    match action {
        ReferentialAction::Cascade => sql::ForeignKeyAction::Cascade,
        ReferentialAction::Restrict => sql::ForeignKeyAction::Restrict,
        ReferentialAction::NoAction => sql::ForeignKeyAction::NoAction,
        ReferentialAction::SetNull => sql::ForeignKeyAction::SetNull,
        ReferentialAction::SetDefault => sql::ForeignKeyAction::SetDefault,
    }
}

/// Unwraps the value from dbgenerated() or dbgenerated("something")
fn unwrap_dbgenerated(expr: &ast::Expression) -> Option<String> {
    expr.as_function()
        .unwrap()
        .1
        .arguments
        .get(0)
        .map(|arg| arg.value.as_string_value().unwrap().0.to_owned())
}

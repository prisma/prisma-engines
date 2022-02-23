mod sql_schema_calculator_flavour;

pub(super) use sql_schema_calculator_flavour::SqlSchemaCalculatorFlavour;

use crate::{flavour::SqlFlavour, SqlDatabaseSchema};
use datamodel::{
    datamodel_connector::{walker_ext_traits::*, ReferentialAction, ScalarType},
    dml::PrismaValue,
    parser_database::{
        walkers::{ModelWalker, ScalarFieldWalker},
        IndexAlgorithm, IndexType, ScalarFieldType, SortOrder,
    },
    schema_ast::ast::{self, FieldArity},
    ValidatedSchema,
};
use sql_schema_describer as sql;

pub(crate) fn calculate_sql_schema(datamodel: &ValidatedSchema, flavour: &dyn SqlFlavour) -> SqlDatabaseSchema {
    let mut schema = SqlDatabaseSchema::default();

    schema.describer_schema.enums = flavour.calculate_enums(datamodel);

    let mut context = Context {
        datamodel,
        schema: &mut schema,
        flavour,
    };

    // Two types of tables: model tables and implicit M2M relation tables (a.k.a. join tables.).
    push_model_tables(&mut context);
    push_relation_tables(&mut context);

    schema
}

fn push_model_tables(ctx: &mut Context<'_>) {
    for (model_idx, model) in ctx.datamodel.db.walk_models().enumerate() {
        let columns = model
            .scalar_fields()
            .enumerate()
            .map(|(field_idx, field)| column_for_scalar_field(field, (model_idx, field_idx), ctx))
            .collect();

        let primary_key = model.primary_key().map(|pk| sql::PrimaryKey {
            columns: pk
                .scalar_field_attributes()
                .map(|field| sql::PrimaryKeyColumn {
                    name: field.as_scalar_field().database_name().to_owned(),
                    length: field.length(),
                    sort_order: field.sort_order().map(|so| match so {
                        SortOrder::Asc => sql::SQLSortOrder::Asc,
                        SortOrder::Desc => sql::SQLSortOrder::Desc,
                    }),
                })
                .collect(),
            sequence: None,
            constraint_name: pk
                .constraint_name(ctx.flavour.datamodel_connector())
                .map(|c| c.into_owned()),
        });

        let indices = model
            .indexes()
            .map(|index| {
                let columns = index
                    .scalar_field_attributes()
                    .map(|sf| sql::IndexColumn {
                        name: sf.as_scalar_field().database_name().into(),
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

                let algorithm = index.algorithm().map(|algo| match algo {
                    IndexAlgorithm::BTree => sql::SQLIndexAlgorithm::BTree,
                    IndexAlgorithm::Hash => sql::SQLIndexAlgorithm::Hash,
                });

                sql::Index {
                    name: index.constraint_name(ctx.flavour.datamodel_connector()).into_owned(),
                    // The model index definition uses the model field names, but the SQL Index wants the column names.
                    columns,
                    tpe: index_type,
                    algorithm,
                }
            })
            .collect();

        let mut table = sql::Table {
            name: model.database_name().to_owned(),
            columns,
            indices,
            primary_key,
            foreign_keys: Vec::new(),
        };

        if ctx.datamodel.referential_integrity().uses_foreign_keys() {
            push_inline_relations(model, &mut table, ctx);
        }

        ctx.schema.describer_schema.tables.push(table);
    }
}

fn push_inline_relations(model: ModelWalker<'_>, table: &mut sql::Table, ctx: &mut Context<'_>) {
    let relations = model.relations_from().filter_map(|r| r.refine().as_inline());

    for relation in relations {
        let relation_field = relation
            .forward_relation_field()
            .expect("Expecting a complete relation in sql_schmea_calculator");
        let fk_columns: Vec<String> = relation_field
            .referencing_fields()
            .expect("Expecting a relation fields in sql_schmea_calculator")
            .map(|rf| rf.database_name().to_owned())
            .collect();
        let on_delete_action = relation_field.explicit_on_delete().unwrap_or_else(|| {
            relation_field.default_on_delete_action(
                ctx.datamodel.configuration.referential_integrity().unwrap_or_default(),
                ctx.flavour.datamodel_connector(),
            )
        });

        table.foreign_keys.push(sql::ForeignKey {
            constraint_name: Some(relation.constraint_name(ctx.flavour.datamodel_connector()).into_owned()),
            columns: fk_columns,
            referenced_table: relation.referenced_model().database_name().to_owned(),
            referenced_columns: relation_field
                .referenced_fields()
                .expect("Expected references to be defined on relation field")
                .map(|f| f.database_name().to_owned())
                .collect(),
            on_update_action: relation_field
                .explicit_on_update()
                .map(convert_referential_action)
                .unwrap_or_else(|| sql::ForeignKeyAction::Cascade),
            on_delete_action: convert_referential_action(on_delete_action),
        });
    }
}

fn push_relation_tables(ctx: &mut Context<'_>) {
    let max_identifier_length = ctx.datamodel.configuration.max_identifier_length();
    let datamodel = ctx.datamodel;
    let flavour = ctx.flavour;
    let m2m_relations = datamodel
        .db
        .walk_relations()
        .filter_map(|relation| relation.refine().as_many_to_many());

    for m2m in m2m_relations {
        let table_name = format!("_{}", m2m.relation_name());
        let model_a = m2m.model_a();
        let model_b = m2m.model_b();
        let model_a_column = "A";
        let model_b_column = "B";
        let model_a_id = model_a.primary_key().unwrap().fields().next().unwrap();
        let model_b_id = model_b.primary_key().unwrap().fields().next().unwrap();

        let foreign_keys = if ctx.datamodel.referential_integrity().uses_foreign_keys() {
            vec![
                sql::ForeignKey {
                    constraint_name: None,
                    columns: vec![model_a_column.into()],
                    referenced_table: model_a.database_name().into(),
                    referenced_columns: vec![model_a_id.database_name().into()],
                    on_update_action: flavour.m2m_foreign_key_action(model_a, model_b),
                    on_delete_action: flavour.m2m_foreign_key_action(model_a, model_b),
                },
                sql::ForeignKey {
                    constraint_name: None,
                    columns: vec![model_b_column.into()],
                    referenced_table: model_b.database_name().into(),
                    referenced_columns: vec![model_b_id.database_name().into()],
                    on_update_action: flavour.m2m_foreign_key_action(model_a, model_b),
                    on_delete_action: flavour.m2m_foreign_key_action(model_a, model_b),
                },
            ]
        } else {
            Vec::new()
        };

        let indexes = vec![
            sql::Index {
                name: format!(
                    "{}_AB_unique",
                    table_name.chars().take(max_identifier_length - 10).collect::<String>()
                ),
                columns: vec![
                    sql::IndexColumn::new(model_a_column),
                    sql::IndexColumn::new(model_b_column),
                ],
                tpe: sql::IndexType::Unique,
                algorithm: None,
            },
            sql::Index {
                name: format!(
                    "{}_B_index",
                    table_name.chars().take(max_identifier_length - 8).collect::<String>()
                ),
                columns: vec![sql::IndexColumn::new(model_b_column)],
                tpe: sql::IndexType::Normal,
                algorithm: None,
            },
        ];

        let columns = vec![
            sql::Column {
                name: model_a_column.into(),
                tpe: column_for_scalar_field(model_a_id, (0, 0), ctx).tpe,
                default: None,
                auto_increment: false,
            },
            sql::Column {
                name: model_b_column.into(),
                tpe: column_for_scalar_field(model_b_id, (0, 0), ctx).tpe,
                default: None,
                auto_increment: false,
            },
        ];

        ctx.schema.describer_schema.tables.push(sql::Table {
            name: table_name
                .chars()
                .take(datamodel.configuration.max_identifier_length())
                .collect::<String>(),
            columns,
            indices: indexes,
            primary_key: None,
            foreign_keys,
        });
    }
}

fn column_for_scalar_field(field: ScalarFieldWalker<'_>, idx: (usize, usize), ctx: &mut Context<'_>) -> sql::Column {
    match field.resolved_scalar_field_type() {
        ScalarFieldType::Enum(enum_id) => column_for_model_enum_scalar_field(field, enum_id, ctx),
        ScalarFieldType::CompositeType(_) => column_for_builtin_scalar_type(field, ScalarType::Json, idx, ctx),
        ScalarFieldType::BuiltInScalar(scalar_type) => column_for_builtin_scalar_type(field, scalar_type, idx, ctx),
        ScalarFieldType::Unsupported(_) => column_for_model_unsupported_scalar_field(field, ctx),
        ScalarFieldType::Alias(_) => unreachable!(),
    }
}

fn column_for_model_enum_scalar_field(
    field: ScalarFieldWalker<'_>,
    enum_id: ast::EnumId,
    ctx: &mut Context<'_>,
) -> sql::Column {
    let r#enum = ctx.datamodel.db.walk_enum(enum_id);
    let default = field.default_value().and_then(|def| match def.value() {
        ast::Expression::ConstantValue(value_name, _) => {
            let value = r#enum
                .values()
                .find(|v| v.name() == value_name)
                .expect("Expected enum field default to reference existing value.");

            let def = sql::DefaultValue::value(PrismaValue::Enum(value.database_name().to_owned()))
                .with_constraint_name(ctx.flavour.default_constraint_name(def));
            Some(def)
        }
        other => unwrap_dbgenerated(other).map(|value| {
            sql::DefaultValue::db_generated(value).with_constraint_name(ctx.flavour.default_constraint_name(def))
        }),
    });
    sql::Column {
        name: field.database_name().to_owned(),
        tpe: ctx.flavour.enum_column_type(field, r#enum.database_name()),
        default,
        auto_increment: false,
    }
}

fn column_for_model_unsupported_scalar_field(field: ScalarFieldWalker<'_>, ctx: &mut Context<'_>) -> sql::Column {
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
    }
}

fn column_for_builtin_scalar_type(
    field: ScalarFieldWalker<'_>,
    scalar_type: ScalarType,
    idx: (usize, usize),
    ctx: &mut Context<'_>,
) -> sql::Column {
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

    let tpe = sql::ColumnType {
        family,
        full_data_type: String::new(),
        arity: column_arity(field.ast_field().arity),
        native_type: Some(native_type),
    };

    let default = field.default_value().and_then(|v| {
        let default_value = {
            if v.is_dbgenerated() {
                unwrap_dbgenerated(v.value()).map(|v| sql::DefaultValue::new(sql::DefaultKind::DbGenerated(v)))
            } else if v.is_now() {
                Some(sql::DefaultValue::now())
            } else if v.is_autoincrement() {
                Some(sql::DefaultValue::sequence(""))
            } else {
                match v.value() {
                    ast::Expression::NumericValue(val, _) => match scalar_type {
                        ScalarType::Int => Some(sql::DefaultValue::new(sql::DefaultKind::Value(PrismaValue::Int(
                            val.parse().unwrap(),
                        )))),
                        ScalarType::BigInt => Some(sql::DefaultValue::new(sql::DefaultKind::Value(
                            PrismaValue::BigInt(val.parse().unwrap()),
                        ))),
                        ScalarType::Float | ScalarType::Decimal => Some(sql::DefaultValue::new(
                            sql::DefaultKind::Value(PrismaValue::Float(val.parse().unwrap())),
                        )),
                        other => unreachable!("{:?}", other),
                    },
                    ast::Expression::StringValue(val, _) => match scalar_type {
                        ScalarType::String => Some(sql::DefaultValue::new(sql::DefaultKind::Value(
                            PrismaValue::String(val.clone()),
                        ))),
                        ScalarType::DateTime => Some(sql::DefaultValue::new(sql::DefaultKind::Value(
                            PrismaValue::DateTime(val.parse().unwrap()),
                        ))),
                        ScalarType::Json => Some(sql::DefaultValue::new(sql::DefaultKind::Value(PrismaValue::Json(
                            val.parse().unwrap(),
                        )))),
                        ScalarType::Bytes => Some(sql::DefaultValue::new(sql::DefaultKind::Value(PrismaValue::Bytes(
                            datamodel::prisma_value::decode_bytes(val).unwrap(),
                        )))),
                        ScalarType::Decimal => Some(sql::DefaultValue::new(sql::DefaultKind::Value(
                            PrismaValue::Float(val.parse().unwrap()),
                        ))),
                        other => unreachable!("{:?}", other),
                    },

                    // The only case where we have constant defaults in scalars is booleans.
                    ast::Expression::ConstantValue(val, _) => Some(sql::DefaultValue::new(sql::DefaultKind::Value(
                        PrismaValue::Boolean(val.parse().unwrap()),
                    ))),
                    ast::Expression::Function(_, _, _) => {
                        // prisma-generated
                        ctx.schema.prisma_level_defaults.push((idx.0 as u32, idx.1 as u32));
                        return None;
                    }
                    ast::Expression::Array(_, _) => unreachable!("Array defaults are not implemented"),
                }
            }
        };

        default_value.map(|df| df.with_constraint_name(ctx.flavour.default_constraint_name(v)))
    });

    sql::Column {
        name: field.database_name().to_owned(),
        tpe,
        default,
        auto_increment: field.is_autoincrement() || ctx.flavour.field_is_implicit_autoincrement_primary_key(field),
    }
}

fn column_arity(arity: FieldArity) -> sql::ColumnArity {
    match &arity {
        FieldArity::Required => sql::ColumnArity::Required,
        FieldArity::List => sql::ColumnArity::List,
        FieldArity::Optional => sql::ColumnArity::Nullable,
    }
}

struct Context<'a> {
    datamodel: &'a ValidatedSchema,
    schema: &'a mut SqlDatabaseSchema,
    flavour: &'a dyn SqlFlavour,
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

/// Unwraps the value from dbgenerated() or dbgenerated("something")
fn unwrap_dbgenerated(expr: &ast::Expression) -> Option<String> {
    expr.as_function()
        .unwrap()
        .1
        .arguments
        .get(0)
        .map(|arg| arg.value.as_string_value().unwrap().0.to_owned())
}

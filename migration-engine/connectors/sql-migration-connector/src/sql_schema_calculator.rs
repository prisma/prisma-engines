mod sql_schema_calculator_flavour;

pub(super) use sql_schema_calculator_flavour::SqlSchemaCalculatorFlavour;

use crate::{flavour::SqlFlavour, sql_renderer::IteratorJoin};
use datamodel::{
    walkers::{walk_models, walk_relations, ModelWalker, RelationFieldWalker, ScalarFieldWalker, TypeWalker},
    Datamodel, DefaultValue, FieldArity, IndexDefinition, IndexType, ScalarType,
};
use prisma_value::PrismaValue;
use sql_schema_describer::{self as sql, walkers::SqlSchemaExt, ColumnType};

pub(crate) fn calculate_sql_schema(datamodel: &Datamodel, flavour: &dyn SqlFlavour) -> sql::SqlSchema {
    let mut schema = sql::SqlSchema::empty();

    schema.enums = flavour.calculate_enums(datamodel);

    // Two types of tables: model tables and implicit M2M relation tables (a.k.a. join tables.).
    schema.tables.extend(calculate_model_tables(datamodel, flavour));

    let relation_tables: Vec<_> = calculate_relation_tables(datamodel, flavour, &schema).collect();
    schema.tables.extend(relation_tables.into_iter());

    schema
}

fn calculate_model_tables<'a>(
    datamodel: &'a Datamodel,
    flavour: &'a dyn SqlFlavour,
) -> impl Iterator<Item = sql::Table> + 'a {
    walk_models(datamodel).map(move |model| {
        // Throughout this function, we reloy on the following invariant: the
        // columns in the SQL table match 1:1 with the scalar fields in the model, and they are
        // in the same order.
        let columns: Vec<_> = model
            .scalar_fields()
            .map(|field| column_for_scalar_field(&field, flavour))
            .collect();

        let mut indexes = Vec::new();

        let primary_key = Some(sql::PrimaryKey {
            columns: model.id_fields().map(|field| field.db_name().to_owned()).collect(),
            sequence: None,
            constraint_name: None,
        })
        .filter(|pk| !pk.columns.is_empty());

        let single_field_indexes = model
            .scalar_fields()
            .enumerate()
            .filter(|(_, f)| f.is_unique())
            .map(|(idx, f)| sql::Index {
                name: flavour.single_field_index_name(model.db_name(), f.db_name()),
                columns: vec![idx],
                tpe: sql::IndexType::Unique,
            });

        let multiple_field_indexes = model.indexes().map(|index_definition: &IndexDefinition| {
            let index_columns: Vec<usize> = index_definition
                .fields
                .iter()
                .map(|field_name| {
                    model
                        .index_of_scalar_field(field_name)
                        .expect("Unknown field in index directive.")
                })
                .collect();

            let index_type = match index_definition.tpe {
                IndexType::Unique => sql::IndexType::Unique,
                IndexType::Normal => sql::IndexType::Normal,
            };

            let index_name = index_definition.name.clone().unwrap_or_else(|| {
                format!(
                    "{table}.{fields}_{qualifier}",
                    table = &model.db_name(),
                    fields = index_columns.iter().map(|idx| columns[*idx].name.as_str()).join("_"),
                    qualifier = if index_type.is_unique() { "unique" } else { "index" },
                )
            });

            sql::Index {
                name: index_name,
                // The model index definition uses the model field names, but the SQL Index
                // wants the column names.
                columns: index_columns,
                tpe: index_type,
            }
        });

        indexes.extend(single_field_indexes.chain(multiple_field_indexes));

        let mut table = sql::Table {
            name: model.database_name().to_owned(),
            columns,
            indices: indexes, // -.-
            primary_key,
            foreign_keys: Vec::new(),
        };

        push_inline_relations(model, &mut table);

        table
    })
}

fn push_inline_relations(model: ModelWalker<'_>, table: &mut sql::Table) {
    let relation_fields = model
        .relation_fields()
        .filter(|relation_field| !relation_field.is_virtual());

    for relation_field in relation_fields {
        // Optional unique index for 1:1 relations.
        if relation_field.is_one_to_one() {
            push_one_to_one_relation_unique_index(&relation_field, table);
        }

        // Foreign key
        let fk = sql::ForeignKey {
            constraint_name: None,
            columns: relation_field
                .constrained_fields()
                .map(|field| field.db_name().to_owned())
                .collect(),
            referenced_table: relation_field.referenced_model().database_name().to_owned(),
            referenced_columns: relation_field.referenced_columns().map(String::from).collect(),
            on_update_action: sql::ForeignKeyAction::Cascade,
            on_delete_action: match column_arity(relation_field.arity()) {
                sql::ColumnArity::Required => sql::ForeignKeyAction::Cascade,
                _ => sql::ForeignKeyAction::SetNull,
            },
        };

        table.foreign_keys.push(fk);
    }
}

fn push_one_to_one_relation_unique_index(relation_field: &RelationFieldWalker<'_>, table: &mut sql::Table) {
    let column_indexes = relation_field
        .constrained_fields()
        .map(|field| {
            relation_field
                .model()
                .scalar_fields()
                .position(|f| f.field_index() == field.field_index())
                .expect("Invariant violation: relation field cannot find constrained fields on parent model.")
        })
        .collect();

    // Don't add a duplicate index.
    if table
        .indices
        .iter()
        .any(|index| index.columns == column_indexes && index.tpe.is_unique())
    {
        return;
    }

    // Don't add if there is a @@id or @id covering
    if let Some(pk) = &table.primary_key {
        if pk.columns == relation_field.constrained_field_names() {
            return;
        }
    }

    let columns_suffix = relation_field.constrained_field_names().join("_");

    let index = sql::Index {
        name: format!("{}_{}_unique", table.name, columns_suffix),
        columns: column_indexes,
        tpe: sql::IndexType::Unique,
    };

    table.indices.push(index);
}

fn calculate_relation_tables<'a>(
    datamodel: &'a Datamodel,
    flavour: &'a dyn SqlFlavour,
    schema: &'a sql::SqlSchema,
) -> impl Iterator<Item = sql::Table> + 'a {
    walk_relations(datamodel)
        .filter_map(|relation| relation.as_m2m())
        .map(move |m2m| {
            let table_name = m2m.table_name();
            let model_a_id = m2m.model_a_id();
            let model_b_id = m2m.model_b_id();
            let model_a = model_a_id.model();
            let model_b = model_b_id.model();

            let foreign_keys = vec![
                sql::ForeignKey {
                    constraint_name: None,
                    columns: vec![m2m.model_a_column().into()],
                    referenced_table: model_a.db_name().into(),
                    referenced_columns: vec![model_a_id.db_name().into()],
                    on_update_action: flavour.m2m_foreign_key_action(&model_a, &model_b),
                    on_delete_action: flavour.m2m_foreign_key_action(&model_a, &model_b),
                },
                sql::ForeignKey {
                    constraint_name: None,
                    columns: vec![m2m.model_b_column().into()],
                    referenced_table: model_b.db_name().into(),
                    referenced_columns: vec![model_b_id.db_name().into()],
                    on_update_action: flavour.m2m_foreign_key_action(&model_a, &model_b),
                    on_delete_action: flavour.m2m_foreign_key_action(&model_a, &model_b),
                },
            ];

            let indexes = vec![
                sql::Index {
                    name: format!("{}_AB_unique", &table_name),
                    columns: vec![0, 1],
                    tpe: sql::IndexType::Unique,
                },
                sql::Index {
                    name: format!("{}_B_index", &table_name),
                    columns: vec![1],
                    tpe: sql::IndexType::Normal,
                },
            ];

            let columns = vec![
                sql::Column {
                    name: m2m.model_a_column().into(),
                    tpe: column_type_for_implicit_relation(&model_a_id, schema),
                    default: None,
                    auto_increment: false,
                },
                sql::Column {
                    name: m2m.model_b_column().into(),
                    tpe: column_type_for_implicit_relation(&model_b_id, schema),
                    default: None,
                    auto_increment: false,
                },
            ];

            sql::Table {
                name: table_name,
                columns,
                indices: indexes,
                primary_key: None,
                foreign_keys,
            }
        })
}

fn column_type_for_implicit_relation(id_field: &ScalarFieldWalker<'_>, schema: &sql::SqlSchema) -> sql::ColumnType {
    let referenced_model = id_field.model();

    schema
        .table_walker(referenced_model.database_name())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Invariant violation: M2M relation field referencing unknown table: {}",
                referenced_model.database_name()
            )
        })
        .unwrap()
        .column(id_field.db_name())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Invariant violation: M2M relation field referencing unknown id field: {}.{}",
                referenced_model.database_name(),
                id_field.db_name()
            )
        })
        .unwrap()
        .column_type()
        .clone()
}

fn column_for_scalar_field(field: &ScalarFieldWalker<'_>, flavour: &dyn SqlFlavour) -> sql::Column {
    let (scalar_type, native_type) = match field.field_type() {
        // Special-case enums. Defaults and type are handled differently.
        TypeWalker::Enum(r#enum) => {
            return sql::Column {
                name: field.db_name().to_owned(),
                tpe: flavour.enum_column_type(field, r#enum.db_name()),
                default: field
                    .default_value()
                    .and_then(|default| default.as_single().and_then(|v| v.as_enum_value()))
                    .map(|value| {
                        let corresponding_value = r#enum.value(value).expect("Could not find enum value");

                        sql::DefaultValue::value(PrismaValue::Enum(
                            corresponding_value.final_database_name().to_owned(),
                        ))
                    }),
                auto_increment: false,
            }
        }
        TypeWalker::Base(scalar_type) => (scalar_type, flavour.default_native_type_for_scalar_type(&scalar_type)),
        TypeWalker::NativeType(scalar_type, instance) => (scalar_type, instance.serialized_native_type.clone()),
        TypeWalker::Unsupported(description) => {
            return sql::Column {
                name: field.db_name().to_owned(),
                tpe: ColumnType {
                    full_data_type: String::new(),
                    native_type: None,
                    family: sql::ColumnTypeFamily::Unsupported(description),
                    arity: column_arity(field.arity()),
                },
                default: field.default_value().and_then(|v| db_generated(v)),
                auto_increment: false,
            }
        }
    };

    let has_auto_increment_default = field
        .default_value()
        .map(|default| default.is_autoincrement())
        .unwrap_or(false);

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

    sql::Column {
        auto_increment: has_auto_increment_default || flavour.field_is_implicit_autoincrement_primary_key(field),
        name: field.db_name().to_owned(),
        tpe: sql::ColumnType {
            full_data_type: String::new(),
            native_type: Some(native_type),
            family,
            arity: column_arity(field.arity()),
        },
        default: field.default_value().and_then(|v| match v {
            datamodel::DefaultValue::Single(v) => Some(sql::DefaultValue::value(v.clone())),
            default if default.is_dbgenerated() => db_generated(default),
            default if default.is_now() => Some(sql::DefaultValue::now()),
            default if default.is_autoincrement() => Some(sql::DefaultValue::sequence(String::new())),
            datamodel::DefaultValue::Expression(_) => None,
        }),
    }
}

fn column_arity(arity: FieldArity) -> sql::ColumnArity {
    match &arity {
        FieldArity::Required => sql::ColumnArity::Required,
        FieldArity::List => sql::ColumnArity::List,
        FieldArity::Optional => sql::ColumnArity::Nullable,
    }
}

fn db_generated(default: &DefaultValue) -> Option<sql::DefaultValue> {
    default.db_generated_description().map(sql::DefaultValue::db_generated)
}

mod sql_schema_calculator_flavour;

pub(super) use sql_schema_calculator_flavour::SqlSchemaCalculatorFlavour;

use crate::flavour::SqlFlavour;
use datamodel::{
    walkers::{walk_models, walk_relations, ModelWalker, ScalarFieldWalker, TypeWalker},
    Configuration, Datamodel, DefaultValue, FieldArity, IndexDefinition, IndexType, ScalarType,
};
use prisma_value::PrismaValue;
use sql_schema_describer::{self as sql, walkers::SqlSchemaExt, ColumnType};

pub(crate) fn calculate_sql_schema(
    (configuration, datamodel): (&Configuration, &Datamodel),
    flavour: &dyn SqlFlavour,
) -> sql::SqlSchema {
    let mut schema = sql::SqlSchema::empty();

    schema.enums = flavour.calculate_enums(datamodel);

    // Two types of tables: model tables and implicit M2M relation tables (a.k.a. join tables.).
    schema.tables.extend(calculate_model_tables(datamodel, flavour));

    let mut relation_tables: Vec<_> = calculate_relation_tables(datamodel, flavour, &schema).collect();
    schema.tables.append(&mut relation_tables);

    let referential_integrity = configuration.referential_integrity().unwrap_or_default();

    if !referential_integrity.uses_foreign_keys() {
        for table in &mut schema.tables {
            table.foreign_keys.clear();
        }
    }

    schema
}

fn calculate_model_tables<'a>(
    datamodel: &'a Datamodel,
    flavour: &'a dyn SqlFlavour,
) -> impl Iterator<Item = sql::Table> + 'a {
    walk_models(datamodel).map(move |model| {
        let columns = model
            .scalar_fields()
            .map(|field| column_for_scalar_field(&field, flavour))
            .collect();

        let primary_key = model.get().primary_key.as_ref().map(|pk| sql::PrimaryKey {
            columns: pk
                .fields
                .iter()
                .map(|field| model.find_scalar_field(field).unwrap().db_name().to_string())
                .collect(),
            sequence: None,
            constraint_name: pk.db_name.clone(),
        });

        let indices = model
            .indexes()
            .map(|index_definition: &IndexDefinition| {
                let referenced_fields: Vec<ScalarFieldWalker<'_>> = index_definition
                    .fields
                    .iter()
                    .map(|field_name| {
                        model
                            .find_scalar_field(field_name)
                            .expect("Unknown field in index directive.")
                    })
                    .collect();

                let index_type = match index_definition.tpe {
                    IndexType::Unique => sql::IndexType::Unique,
                    IndexType::Normal => sql::IndexType::Normal,
                };

                sql::Index {
                    name: index_definition.db_name.clone().unwrap(),
                    // The model index definition uses the model field names, but the SQL Index wants the column names.
                    columns: referenced_fields
                        .iter()
                        .map(|field| field.db_name().to_owned())
                        .collect(),
                    tpe: index_type,
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

        push_inline_relations(model, &mut table, flavour);

        table
    })
}

fn push_inline_relations(model: ModelWalker<'_>, table: &mut sql::Table, flavour: &dyn SqlFlavour) {
    let relation_fields = model
        .relation_fields()
        .filter(|relation_field| !relation_field.is_virtual());

    for relation_field in relation_fields {
        let fk_columns: Vec<String> = relation_field.referencing_columns().map(String::from).collect();

        // Foreign key
        {
            let fk = sql::ForeignKey {
                constraint_name: relation_field.constraint_name(),
                columns: fk_columns,
                referenced_table: relation_field.referenced_model().database_name().to_owned(),
                referenced_columns: relation_field.referenced_columns().map(String::from).collect(),
                on_update_action: flavour.on_update_action(&relation_field),
                on_delete_action: flavour.on_delete_action(&relation_field),
            };

            table.foreign_keys.push(fk);
        }
    }
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
                    columns: vec![m2m.model_a_column().into(), m2m.model_b_column().into()],
                    tpe: sql::IndexType::Unique,
                },
                sql::Index {
                    name: format!("{}_B_index", &table_name),
                    columns: vec![m2m.model_b_column().into()],
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
            format!(
                "Invariant violation: M2M relation field referencing unknown table: {}",
                referenced_model.database_name()
            )
        })
        .unwrap()
        .column(id_field.db_name())
        .ok_or_else(|| {
            format!(
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
                    .and_then(|default| {
                        let as_enum = default.as_single().and_then(|v| v.as_enum_value());
                        as_enum.map(|enm| (enm, default.db_name()))
                    })
                    .map(|(value, db_name)| {
                        let corresponding_value = r#enum.value(value).expect("Could not find enum value");

                        let mut default = sql::DefaultValue::value(PrismaValue::Enum(
                            corresponding_value.final_database_name().to_owned(),
                        ));

                        if let Some(db_name) = db_name {
                            default.set_constraint_name(db_name);
                        }

                        default
                    }),
                auto_increment: false,
            }
        }
        TypeWalker::Base(scalar_type) => (scalar_type, flavour.default_native_type_for_scalar_type(&scalar_type)),
        TypeWalker::NativeType(scalar_type, instance) => (scalar_type, instance.serialized_native_type.clone()),
        TypeWalker::Unsupported(description) => {
            let default = field.default_value().and_then(|v| db_generated(v)).map(|mut default| {
                if let Some(name) = field.default_value().and_then(|v| v.db_name()) {
                    default.set_constraint_name(name);
                }

                default
            });

            return sql::Column {
                name: field.db_name().to_owned(),
                tpe: ColumnType {
                    full_data_type: String::new(),
                    native_type: None,
                    family: sql::ColumnTypeFamily::Unsupported(description),
                    arity: column_arity(field.arity()),
                },
                default,
                auto_increment: false,
            };
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

    let default = field.default_value().and_then(|v| {
        let mut df = match v.kind() {
            datamodel::DefaultKind::Single(v) => Some(sql::DefaultValue::value(v.clone())),
            default if default.is_dbgenerated() => db_generated(v),
            default if default.is_now() => Some(sql::DefaultValue::now()),
            default if default.is_autoincrement() => Some(sql::DefaultValue::sequence(String::new())),
            datamodel::DefaultKind::Expression(_) => None,
        };

        if let (Some(df), Some(db_name)) = (df.as_mut(), v.db_name()) {
            df.set_constraint_name(db_name);
        }

        df
    });

    sql::Column {
        auto_increment: has_auto_increment_default || flavour.field_is_implicit_autoincrement_primary_key(field),
        name: field.db_name().to_owned(),
        tpe: sql::ColumnType {
            full_data_type: String::new(),
            native_type: Some(native_type),
            family,
            arity: column_arity(field.arity()),
        },
        default,
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

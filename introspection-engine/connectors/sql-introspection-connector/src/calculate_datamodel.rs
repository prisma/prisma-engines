use crate::commenting_out_guardrails::commenting_out_guardrails;
use crate::introspection::introspect;
use crate::introspection_helpers::*;
use crate::prisma_1_defaults::*;
use crate::re_introspection::enrich;
use crate::sanitize_datamodel_names::{sanitization_leads_to_duplicate_names, sanitize_datamodel_names};
use crate::version_checker::VersionChecker;
use crate::SqlIntrospectionResult;
use datamodel::common::datamodel_context::DatamodelContext;
use datamodel::Datamodel;
use introspection_connector::IntrospectionResult;
use quaint::connector::SqlFamily;
use sql_schema_describer::*;
use tracing::debug;

/// Calculate a data model from a database schema.
pub fn calculate_datamodel(
    schema: &SqlSchema,
    previous_data_model: &Datamodel,
    ctx: &DatamodelContext,
) -> SqlIntrospectionResult<IntrospectionResult> {
    debug!("Calculating data model.");

    let family = match ctx.connector.name() {
        "Postgres" => SqlFamily::Postgres,
        "sqlite" => SqlFamily::Sqlite,
        "SQL Server" => SqlFamily::Mssql,
        "MySQL" => SqlFamily::Mysql,
        name => unreachable!(format!("The name `{}` for the datamodel connector is not known", name)),
    };

    let mut version_check = VersionChecker::new(family, schema);
    let mut data_model = Datamodel::new();

    // 1to1 translation of the sql schema
    introspect(schema, &mut version_check, &mut data_model, ctx)?;

    if !sanitization_leads_to_duplicate_names(&data_model) {
        // our opinionation about valid names
        sanitize_datamodel_names(&mut data_model, &family);
    }

    // deduplicating relation field names
    deduplicate_relation_field_names(&mut data_model);

    let mut warnings = vec![];
    if !previous_data_model.is_empty() {
        warnings.append(&mut enrich(previous_data_model, &mut data_model, &family));
        tracing::debug!("Enriching datamodel is done: {:?}", data_model);
    }

    // commenting out models, fields, enums, enum values
    warnings.append(&mut commenting_out_guardrails(&mut data_model, &family));

    // try to identify whether the schema was created by a previous Prisma version
    let version = version_check.version(&warnings, &data_model);

    // if based on a previous Prisma version add id default opinionations
    add_prisma_1_id_defaults(&family, &version, &mut data_model, schema, &mut warnings);

    // renderer -> parser -> validator, is_commented_out gets lost between renderer and parser
    debug!("Done calculating data model {:?}", data_model);
    Ok(IntrospectionResult {
        data_model,
        warnings,
        version,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use datamodel::IndexType::Unique;
    use datamodel::{
        dml, Datamodel, DefaultValue as DMLDefault, Field, FieldArity, FieldType, IndexDefinition,
        IndexType as DMLIndexType, Model, NativeTypeInstance, OnDeleteStrategy, PrimaryKeyDefinition, RelationField,
        RelationInfo, ScalarField, ScalarType, ValueGenerator,
    };
    use native_types::{NativeType, PostgresType};
    use pretty_assertions::assert_eq;
    use sql_datamodel_connector::PostgresDatamodelConnector;

    fn ctx() -> DatamodelContext {
        DatamodelContext {
            source_name: Some("Postgres".into()),
            preview_features: vec![],
            connector: Box::new(PostgresDatamodelConnector::new()),
        }
    }

    #[test]
    fn arity_is_preserved_when_generating_data_model_from_a_schema() {
        let ref_data_model = Datamodel {
            models: vec![Model {
                database_name: None,
                name: "Table1".to_string(),
                documentation: None,
                is_embedded: false,
                is_commented_out: false,
                is_ignored: false,
                fields: vec![
                    Field::ScalarField(ScalarField::new(
                        "optional",
                        FieldArity::Optional,
                        FieldType::Base(ScalarType::Int, None),
                    )),
                    Field::ScalarField(ScalarField {
                        name: "required".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::Int, None),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: None,
                        primary_key: Some(PrimaryKeyDefinition {
                            name_in_client: None,
                            name_in_db_matches_default: false,
                            name_in_db: None,
                            fields: vec!["required".to_string()],
                        }),
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }),
                    Field::ScalarField(ScalarField::new(
                        "list",
                        FieldArity::List,
                        FieldType::Base(ScalarType::Int, None),
                    )),
                ],
                is_generated: false,
                indices: vec![],
                primary_key: Some(PrimaryKeyDefinition {
                    name_in_db: None,
                    name_in_db_matches_default: false,
                    name_in_client: None,
                    fields: vec!["required".to_string()],
                }),
            }],
            enums: vec![],
        };

        let schema = SqlSchema {
            procedures: vec![],
            tables: vec![Table {
                name: "Table1".to_string(),
                columns: vec![
                    Column {
                        name: "optional".to_string(),
                        tpe: ColumnType::pure(ColumnTypeFamily::Int, ColumnArity::Nullable),
                        default: None,
                        auto_increment: false,
                    },
                    Column {
                        name: "required".to_string(),
                        tpe: ColumnType::pure(ColumnTypeFamily::Int, ColumnArity::Required),
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "list".to_string(),
                        tpe: ColumnType::pure(ColumnTypeFamily::Int, ColumnArity::List),
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["required".to_string()],
                    sequence: None,
                    constraint_name: None,
                }),
                foreign_keys: vec![],
            }],
            enums: vec![],
            sequences: vec![],
            views: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }

    #[test]
    fn primary_key_is_preserved_when_generating_data_model_from_a_schema() {
        let ref_data_model = Datamodel {
            models: vec![
                // Model with auto-incrementing primary key
                Model {
                    database_name: None,
                    name: "Table1".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![Field::ScalarField(ScalarField {
                        name: "primary".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::NativeType(
                            ScalarType::Int,
                            NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            },
                        ),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: None,
                        primary_key: Some(PrimaryKeyDefinition {
                            name_in_client: None,
                            name_in_db_matches_default: false,
                            name_in_db: None,
                            fields: vec!["primary".to_string()],
                        }),
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    })],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_db: None,
                        name_in_db_matches_default: false,
                        name_in_client: None,
                        fields: vec!["primary".to_string()],
                    }),
                },
                // Model with non-auto-incrementing primary key
                Model {
                    database_name: None,
                    name: "Table2".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![Field::ScalarField(ScalarField {
                        name: "primary".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::NativeType(
                            ScalarType::Int,
                            NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            },
                        ),
                        database_name: None,
                        default_value: None,
                        is_unique: None,
                        primary_key: Some(PrimaryKeyDefinition {
                            name_in_client: None,
                            name_in_db_matches_default: false,
                            name_in_db: None,
                            fields: vec!["primary".to_string()],
                        }),
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    })],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_db: None,
                        name_in_db_matches_default: false,
                        name_in_client: None,
                        fields: vec!["primary".to_string()],
                    }),
                },
                // Model with primary key seeded by sequence
                Model {
                    database_name: None,
                    name: "Table3".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![Field::ScalarField(ScalarField {
                        name: "primary".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::NativeType(
                            ScalarType::Int,
                            NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            },
                        ),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: None,
                        primary_key: Some(PrimaryKeyDefinition {
                            name_in_client: None,
                            name_in_db_matches_default: false,
                            name_in_db: None,
                            fields: vec!["primary".to_string()],
                        }),
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    })],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_db: None,
                        name_in_db_matches_default: false,
                        name_in_client: None,
                        fields: vec!["primary".to_string()],
                    }),
                },
            ],
            enums: vec![],
        };

        let schema = SqlSchema {
            procedures: vec![],
            tables: vec![
                Table {
                    name: "Table1".to_string(),
                    columns: vec![Column {
                        name: "primary".to_string(),
                        tpe: ColumnType {
                            full_data_type: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                            native_type: Some(PostgresType::Integer.to_json()),
                        },
                        default: None,
                        auto_increment: true,
                    }],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["primary".to_string()],
                        sequence: None,
                        constraint_name: None,
                    }),
                    foreign_keys: vec![],
                },
                Table {
                    name: "Table2".to_string(),
                    columns: vec![Column {
                        name: "primary".to_string(),
                        tpe: ColumnType {
                            full_data_type: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                            native_type: Some(PostgresType::Integer.to_json()),
                        },
                        default: None,
                        auto_increment: false,
                    }],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["primary".to_string()],
                        sequence: None,
                        constraint_name: None,
                    }),
                    foreign_keys: vec![],
                },
                Table {
                    name: "Table3".to_string(),
                    columns: vec![Column {
                        name: "primary".to_string(),
                        tpe: ColumnType {
                            full_data_type: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                            native_type: Some(PostgresType::Integer.to_json()),
                        },
                        default: None,
                        auto_increment: true,
                    }],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["primary".to_string()],
                        sequence: Some(Sequence {
                            name: "sequence".to_string(),
                        }),
                        constraint_name: None,
                    }),
                    foreign_keys: vec![],
                },
            ],
            enums: vec![],
            sequences: vec![],
            views: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }

    #[test]
    fn uniqueness_is_preserved_when_generating_data_model_from_a_schema() {
        let ref_data_model = Datamodel {
            models: vec![Model {
                database_name: None,
                name: "Table1".to_string(),
                documentation: None,
                is_embedded: false,
                is_commented_out: false,
                is_ignored: false,
                fields: vec![
                    Field::ScalarField(ScalarField::new(
                        "non_unique",
                        FieldArity::Optional,
                        FieldType::Base(ScalarType::Int, None),
                    )),
                    Field::ScalarField(ScalarField {
                        name: "unique".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::Int, None),
                        database_name: None,
                        default_value: None,
                        is_unique: Some(IndexDefinition {
                            name_in_db: "unique".to_string(),
                            name_in_db_matches_default: false,
                            name_in_client: None,
                            fields: vec!["unique".to_string()],
                            tpe: DMLIndexType::Unique,
                        }),
                        primary_key: None,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }),
                ],
                is_generated: false,
                indices: vec![IndexDefinition {
                    name_in_db: "unique".to_string(),
                    name_in_db_matches_default: false,
                    name_in_client: None,
                    fields: vec!["unique".to_string()],
                    tpe: Unique,
                }],
                primary_key: None,
            }],
            enums: vec![],
        };

        let schema = SqlSchema {
            views: vec![],
            procedures: vec![],
            tables: vec![Table {
                name: "Table1".to_string(),
                columns: vec![
                    Column {
                        name: "non_unique".to_string(),
                        tpe: ColumnType::pure(ColumnTypeFamily::Int, ColumnArity::Nullable),
                        default: None,
                        auto_increment: false,
                    },
                    Column {
                        name: "unique".to_string(),
                        tpe: ColumnType::pure(ColumnTypeFamily::Int, ColumnArity::Required),
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![Index {
                    name: "unique".to_string(),
                    columns: vec!["unique".to_string()],
                    tpe: IndexType::Unique,
                }],
                primary_key: None,
                foreign_keys: vec![],
            }],
            enums: vec![],
            sequences: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }

    #[test]
    fn compound_foreign_keys_are_preserved_when_generating_data_model_from_a_schema() {
        let expected_data_model = Datamodel {
            models: vec![
                Model {
                    database_name: None,
                    name: "City".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![
                        Field::ScalarField(ScalarField {
                            name: "id".to_string(),
                            arity: FieldArity::Required,
                            field_type: FieldType::NativeType(
                                ScalarType::Int,
                                NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                },
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: None,
                            primary_key: Some(PrimaryKeyDefinition {
                                name_in_client: None,
                                name_in_db_matches_default: false,
                                name_in_db: None,
                                fields: vec!["id".to_string()],
                            }),
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField::new(
                            "name",
                            FieldArity::Required,
                            FieldType::NativeType(
                                ScalarType::String,
                                NativeTypeInstance {
                                    name: "Text".into(),
                                    args: Vec::new(),
                                    serialized_native_type: PostgresType::Text.to_json(),
                                },
                            ),
                        )),
                        Field::RelationField(RelationField::new(
                            "User",
                            FieldArity::List,
                            RelationInfo {
                                to: "User".to_string(),
                                fields: vec![],
                                references: vec![],
                                name: "CityToUser".to_string(),
                                on_delete: OnDeleteStrategy::None,
                                fk_name: None,
                                fk_name_matches_default: false,
                            },
                        )),
                    ],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_client: None,
                        name_in_db_matches_default: false,
                        name_in_db: None,
                        fields: vec!["id".to_string()],
                    }),
                },
                Model {
                    database_name: None,
                    name: "User".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![
                        Field::ScalarField(ScalarField {
                            name: "id".to_string(),
                            arity: FieldArity::Required,
                            field_type: FieldType::NativeType(
                                ScalarType::Int,
                                NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                },
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: None,
                            primary_key: Some(PrimaryKeyDefinition {
                                name_in_client: None,
                                name_in_db_matches_default: false,
                                name_in_db: None,
                                fields: vec!["id".to_string()],
                            }),
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField {
                            name: "city_id".to_string(),
                            arity: FieldArity::Required,
                            field_type: FieldType::NativeType(
                                ScalarType::Int,
                                NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                },
                            ),
                            database_name: Some("city-id".to_string()),
                            default_value: None,
                            is_unique: None,
                            primary_key: None,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField {
                            name: "city_name".to_string(),
                            field_type: FieldType::NativeType(
                                ScalarType::String,
                                NativeTypeInstance {
                                    name: "Text".into(),
                                    args: Vec::new(),
                                    serialized_native_type: PostgresType::Text.to_json(),
                                },
                            ),
                            arity: FieldArity::Required,
                            database_name: Some("city-name".to_string()),
                            default_value: None,
                            is_unique: None,
                            primary_key: None,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::RelationField(RelationField::new(
                            "City",
                            FieldArity::Required,
                            RelationInfo {
                                name: "CityToUser".to_string(),
                                to: "City".to_string(),
                                fields: vec!["city_id".to_string(), "city_name".to_string()],
                                references: vec!["id".to_string(), "name".to_string()],
                                on_delete: OnDeleteStrategy::None,
                                fk_name: None,
                                fk_name_matches_default: false,
                            },
                        )),
                    ],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_client: None,
                        name_in_db_matches_default: false,
                        name_in_db: None,
                        fields: vec!["id".to_string()],
                    }),
                },
            ],
            enums: vec![],
        };

        let schema = SqlSchema {
            views: vec![],
            procedures: vec![],
            tables: vec![
                Table {
                    name: "City".to_string(),
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            tpe: ColumnType {
                                full_data_type: "integer".to_string(),
                                family: ColumnTypeFamily::Int,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Integer.to_json()),
                            },
                            default: None,
                            auto_increment: true,
                        },
                        Column {
                            name: "name".to_string(),
                            tpe: ColumnType {
                                full_data_type: "text".to_string(),
                                family: ColumnTypeFamily::String,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Text.to_json()),
                            },
                            default: None,
                            auto_increment: false,
                        },
                    ],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["id".to_string()],
                        sequence: None,
                        constraint_name: None,
                    }),
                    foreign_keys: vec![],
                },
                Table {
                    name: "User".to_string(),
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            tpe: ColumnType {
                                full_data_type: "integer".to_string(),
                                family: ColumnTypeFamily::Int,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Integer.to_json()),
                            },
                            default: None,
                            auto_increment: true,
                        },
                        Column {
                            name: "city-id".to_string(),
                            tpe: ColumnType {
                                full_data_type: "integer".to_string(),
                                family: ColumnTypeFamily::Int,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Integer.to_json()),
                            },
                            default: None,
                            auto_increment: false,
                        },
                        Column {
                            name: "city-name".to_string(),
                            tpe: ColumnType {
                                full_data_type: "text".to_string(),
                                family: ColumnTypeFamily::String,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Text.to_json()),
                            },
                            default: None,
                            auto_increment: false,
                        },
                    ],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["id".to_string()],
                        sequence: None,
                        constraint_name: None,
                    }),
                    foreign_keys: vec![ForeignKey {
                        constraint_name: None,
                        columns: vec!["city-id".to_string(), "city-name".to_string()],
                        referenced_table: "City".to_string(),
                        on_delete_action: ForeignKeyAction::NoAction,
                        on_update_action: ForeignKeyAction::NoAction,
                        referenced_columns: vec!["id".to_string(), "name".to_string()],
                    }],
                },
            ],
            enums: vec![],
            sequences: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, expected_data_model);
    }

    #[test]
    fn multi_field_uniques_are_preserved_when_generating_data_model_from_a_schema() {
        let ref_data_model = Datamodel {
            models: vec![Model {
                database_name: None,
                name: "User".to_string(),
                documentation: None,
                is_embedded: false,
                is_commented_out: false,
                is_ignored: false,
                fields: vec![
                    Field::ScalarField(ScalarField {
                        name: "id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::NativeType(
                            ScalarType::Int,
                            NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            },
                        ),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: None,
                        primary_key: Some(PrimaryKeyDefinition {
                            name_in_client: None,
                            name_in_db_matches_default: false,
                            name_in_db: None,
                            fields: vec!["id".to_string()],
                        }),
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }),
                    Field::ScalarField(ScalarField::new(
                        "name",
                        FieldArity::Required,
                        FieldType::NativeType(
                            ScalarType::String,
                            NativeTypeInstance {
                                name: "Text".into(),
                                args: Vec::new(),
                                serialized_native_type: PostgresType::Text.to_json(),
                            },
                        ),
                    )),
                    Field::ScalarField(ScalarField::new(
                        "lastname",
                        FieldArity::Required,
                        FieldType::NativeType(
                            ScalarType::String,
                            NativeTypeInstance {
                                name: "Text".into(),
                                args: Vec::new(),
                                serialized_native_type: PostgresType::Text.to_json(),
                            },
                        ),
                    )),
                ],
                is_generated: false,
                indices: vec![datamodel::dml::IndexDefinition {
                    name_in_db: "name_last_name_unique".to_string(),
                    name_in_db_matches_default: false,
                    name_in_client: None,
                    fields: vec!["name".to_string(), "lastname".to_string()],
                    tpe: datamodel::dml::IndexType::Unique,
                }],
                primary_key: Some(PrimaryKeyDefinition {
                    name_in_client: None,
                    name_in_db_matches_default: false,
                    name_in_db: None,
                    fields: vec!["id".to_string()],
                }),
            }],
            enums: vec![],
        };

        let schema = SqlSchema {
            views: vec![],
            procedures: vec![],
            tables: vec![Table {
                name: "User".to_string(),
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        tpe: ColumnType {
                            full_data_type: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                            native_type: Some(PostgresType::Integer.to_json()),
                        },
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "name".to_string(),
                        tpe: ColumnType {
                            full_data_type: "text".to_string(),
                            family: ColumnTypeFamily::String,
                            arity: ColumnArity::Required,
                            native_type: Some(PostgresType::Text.to_json()),
                        },
                        default: None,
                        auto_increment: false,
                    },
                    Column {
                        name: "lastname".to_string(),
                        tpe: ColumnType {
                            full_data_type: "text".to_string(),
                            family: ColumnTypeFamily::String,
                            arity: ColumnArity::Required,
                            native_type: Some(PostgresType::Text.to_json()),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![Index {
                    name: "name_last_name_unique".to_string(),
                    columns: vec!["name".to_string(), "lastname".to_string()],
                    tpe: IndexType::Unique,
                }],
                primary_key: Some(PrimaryKey {
                    columns: vec!["id".to_string()],
                    sequence: None,
                    constraint_name: None,
                }),
                foreign_keys: vec![],
            }],
            enums: vec![],
            sequences: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }

    #[test]
    fn foreign_keys_are_preserved_when_generating_data_model_from_a_schema() {
        let ref_data_model = Datamodel {
            models: vec![
                Model {
                    database_name: None,
                    name: "City".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![
                        Field::ScalarField(ScalarField {
                            name: "id".to_string(),
                            arity: FieldArity::Required,
                            field_type: FieldType::NativeType(
                                ScalarType::Int,
                                NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                },
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: None,
                            primary_key: Some(PrimaryKeyDefinition {
                                name_in_client: None,
                                name_in_db_matches_default: false,
                                name_in_db: None,
                                fields: vec!["id".to_string()],
                            }),
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField::new(
                            "name",
                            FieldArity::Required,
                            FieldType::NativeType(
                                ScalarType::String,
                                NativeTypeInstance {
                                    name: "Text".into(),
                                    args: Vec::new(),
                                    serialized_native_type: PostgresType::Text.to_json(),
                                },
                            ),
                        )),
                        Field::RelationField(RelationField::new(
                            "User",
                            FieldArity::List,
                            RelationInfo {
                                to: "User".to_string(),
                                fields: vec![],
                                references: vec![],
                                name: "CityToUser".to_string(),
                                on_delete: OnDeleteStrategy::None,
                                fk_name: None,
                                fk_name_matches_default: false,
                            },
                        )),
                    ],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_client: None,
                        name_in_db_matches_default: false,
                        name_in_db: None,
                        fields: vec!["id".to_string()],
                    }),
                },
                Model {
                    database_name: None,
                    name: "User".to_string(),
                    documentation: None,
                    is_embedded: false,
                    is_commented_out: false,
                    is_ignored: false,
                    fields: vec![
                        Field::ScalarField(ScalarField {
                            name: "id".to_string(),
                            arity: FieldArity::Required,
                            field_type: FieldType::NativeType(
                                ScalarType::Int,
                                NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                },
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: None,
                            primary_key: Some(PrimaryKeyDefinition {
                                name_in_client: None,
                                name_in_db_matches_default: false,
                                name_in_db: None,
                                fields: vec!["id".to_string()],
                            }),
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField::new(
                            "city_id",
                            FieldArity::Required,
                            FieldType::NativeType(
                                ScalarType::Int,
                                NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                },
                            ),
                        )),
                        Field::RelationField(RelationField::new(
                            "City",
                            FieldArity::Required,
                            RelationInfo {
                                name: "CityToUser".to_string(),
                                to: "City".to_string(),
                                fields: vec!["city_id".to_string()],
                                references: vec!["id".to_string()],
                                on_delete: OnDeleteStrategy::None,
                                fk_name: None,
                                fk_name_matches_default: false,
                            },
                        )),
                    ],
                    is_generated: false,
                    indices: vec![],
                    primary_key: Some(PrimaryKeyDefinition {
                        name_in_client: None,
                        name_in_db_matches_default: false,
                        name_in_db: None,
                        fields: vec!["id".to_string()],
                    }),
                },
            ],
            enums: vec![],
        };

        let schema = SqlSchema {
            views: vec![],
            procedures: vec![],
            tables: vec![
                Table {
                    name: "City".to_string(),
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            tpe: ColumnType {
                                full_data_type: "integer".to_string(),
                                family: ColumnTypeFamily::Int,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Integer.to_json()),
                            },
                            default: None,
                            auto_increment: true,
                        },
                        Column {
                            name: "name".to_string(),
                            tpe: ColumnType {
                                full_data_type: "text".to_string(),
                                family: ColumnTypeFamily::String,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Text.to_json()),
                            },
                            default: None,
                            auto_increment: false,
                        },
                    ],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["id".to_string()],
                        sequence: None,
                        constraint_name: None,
                    }),
                    foreign_keys: vec![],
                },
                Table {
                    name: "User".to_string(),
                    columns: vec![
                        Column {
                            name: "id".to_string(),
                            tpe: ColumnType {
                                full_data_type: "integer".to_string(),
                                family: ColumnTypeFamily::Int,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Integer.to_json()),
                            },
                            default: None,
                            auto_increment: true,
                        },
                        Column {
                            name: "city_id".to_string(),
                            tpe: ColumnType {
                                full_data_type: "integer".to_string(),
                                family: ColumnTypeFamily::Int,
                                arity: ColumnArity::Required,
                                native_type: Some(PostgresType::Integer.to_json()),
                            },
                            default: None,
                            auto_increment: false,
                        },
                    ],
                    indices: vec![],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["id".to_string()],
                        sequence: None,
                        constraint_name: None,
                    }),
                    foreign_keys: vec![ForeignKey {
                        constraint_name: None,
                        columns: vec!["city_id".to_string()],
                        referenced_table: "City".to_string(),
                        on_delete_action: ForeignKeyAction::NoAction,
                        on_update_action: ForeignKeyAction::NoAction,
                        referenced_columns: vec!["id".to_string()],
                    }],
                },
            ],
            enums: vec![],
            sequences: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }

    #[test]
    fn enums_are_preserved_when_generating_data_model_from_a_schema() {
        let ref_data_model = Datamodel {
            models: vec![],
            enums: vec![dml::Enum {
                name: "Enum".to_string(),
                database_name: None,
                documentation: None,
                commented_out: false,
                values: vec![
                    datamodel::dml::EnumValue {
                        name: "a".to_string(),
                        documentation: None,
                        database_name: None,
                        commented_out: false,
                    },
                    datamodel::dml::EnumValue {
                        name: "b".to_string(),
                        documentation: None,
                        database_name: None,
                        commented_out: false,
                    },
                ],
            }],
        };

        let enum_values = vec!["a".to_string(), "b".to_string()];
        let schema = SqlSchema {
            views: vec![],
            procedures: vec![],
            tables: vec![],
            enums: vec![Enum {
                name: "Enum".to_string(),
                values: enum_values,
            }],
            sequences: vec![],
            user_defined_types: vec![],
        };
        let introspection_result =
            calculate_datamodel(&schema, &Datamodel::new(), &ctx()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }
}

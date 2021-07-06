#[cfg(test)]
mod tests {
    use crate::calculate_datamodel::calculate_datamodel;
    use datamodel::ast::Span;
    use datamodel::StringFromEnvVar;
    use datamodel::{
        dml, Datamodel, Datasource, DefaultValue as DMLDefault, Field, FieldArity, FieldType, Model,
        NativeTypeInstance, ReferentialAction, RelationField, RelationInfo, ScalarField, ScalarType, ValueGenerator,
    };
    use enumflags2::BitFlags;
    use introspection_connector::IntrospectionContext;
    use native_types::{NativeType, PostgresType};
    use pretty_assertions::assert_eq;
    use sql_datamodel_connector::PostgresDatamodelConnector;
    use sql_schema_describer::{
        Column, ColumnArity, ColumnType, ColumnTypeFamily, Enum, ForeignKey, ForeignKeyAction, Index, IndexType,
        PrimaryKey, Sequence, SqlSchema, Table,
    };

    fn postgres_context() -> IntrospectionContext {
        let source = Datasource {
            name: "Postgres".to_string(),
            active_provider: "postgresql".into(),
            url: StringFromEnvVar::new_literal("test".to_string()),
            url_span: Span::empty(),
            documentation: None,
            active_connector: Box::new(PostgresDatamodelConnector::new()),
            shadow_database_url: None,
            provider: "postgresql".to_string(),
            planet_scale_mode: false,
        };

        IntrospectionContext {
            source,
            preview_features: BitFlags::empty(),
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
                        FieldType::Scalar(ScalarType::Int, None, None),
                    )),
                    Field::ScalarField(ScalarField {
                        name: "required".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Scalar(ScalarType::Int, None, None),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }),
                    Field::ScalarField(ScalarField::new(
                        "list",
                        FieldArity::List,
                        FieldType::Scalar(ScalarType::Int, None, None),
                    )),
                ],
                is_generated: false,
                indices: vec![],
                id_fields: vec![],
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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

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
                        field_type: FieldType::Scalar(
                            ScalarType::Int,
                            None,
                            Some(NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            }),
                        ),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    })],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
                        field_type: FieldType::Scalar(
                            ScalarType::Int,
                            None,
                            Some(NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            }),
                        ),
                        database_name: None,
                        default_value: None,
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    })],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
                        field_type: FieldType::Scalar(
                            ScalarType::Int,
                            None,
                            Some(NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            }),
                        ),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    })],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

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
                        FieldType::Scalar(ScalarType::Int, None, None),
                    )),
                    Field::ScalarField(ScalarField {
                        name: "unique".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Scalar(ScalarType::Int, None, None),
                        database_name: None,
                        default_value: None,
                        is_unique: true,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }),
                ],
                is_generated: false,
                indices: vec![],
                id_fields: vec![],
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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

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
                            field_type: FieldType::Scalar(
                                ScalarType::Int,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                }),
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: false,
                            is_id: true,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField::new(
                            "name",
                            FieldArity::Required,
                            FieldType::Scalar(
                                ScalarType::String,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Text".into(),
                                    args: Vec::new(),
                                    serialized_native_type: PostgresType::Text.to_json(),
                                }),
                            ),
                        )),
                        Field::RelationField(RelationField::new(
                            "User",
                            FieldArity::List,
                            FieldArity::List,
                            RelationInfo {
                                to: "User".to_string(),
                                fields: vec![],
                                references: vec![],
                                name: "CityToUser".to_string(),
                                on_delete: None,
                                on_update: None,
                                legacy_referential_actions: false,
                            },
                        )),
                    ],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
                            field_type: FieldType::Scalar(
                                ScalarType::Int,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                }),
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: false,
                            is_id: true,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField {
                            name: "city_id".to_string(),
                            arity: FieldArity::Required,
                            field_type: FieldType::Scalar(
                                ScalarType::Int,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                }),
                            ),
                            database_name: Some("city-id".to_string()),
                            default_value: None,
                            is_unique: false,
                            is_id: false,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField {
                            name: "city_name".to_string(),
                            field_type: FieldType::Scalar(
                                ScalarType::String,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Text".into(),
                                    args: Vec::new(),
                                    serialized_native_type: PostgresType::Text.to_json(),
                                }),
                            ),
                            arity: FieldArity::Required,
                            database_name: Some("city-name".to_string()),
                            default_value: None,
                            is_unique: false,
                            is_id: false,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::RelationField(RelationField {
                            name: "City".into(),
                            arity: FieldArity::Required,
                            referential_arity: FieldArity::Required,
                            documentation: None,
                            is_generated: false,
                            is_commented_out: false,
                            is_ignored: false,
                            supports_restrict_action: Some(true),
                            emulates_referential_actions: None,
                            relation_info: RelationInfo {
                                name: "CityToUser".to_string(),
                                to: "City".to_string(),
                                fields: vec!["city_id".to_string(), "city_name".to_string()],
                                references: vec!["id".to_string(), "name".to_string()],
                                on_delete: Some(ReferentialAction::NoAction),
                                on_update: Some(ReferentialAction::NoAction),
                                legacy_referential_actions: false,
                            },
                        }),
                    ],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
                        // what does this mean? the from columns are not targeting a specific to column?
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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

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
                        field_type: FieldType::Scalar(
                            ScalarType::Int,
                            None,
                            Some(NativeTypeInstance {
                                name: "Integer".into(),
                                serialized_native_type: PostgresType::Integer.to_json(),
                                args: Vec::new(),
                            }),
                        ),
                        database_name: None,
                        default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                        is_commented_out: false,
                        is_ignored: false,
                    }),
                    Field::ScalarField(ScalarField::new(
                        "name",
                        FieldArity::Required,
                        FieldType::Scalar(
                            ScalarType::String,
                            None,
                            Some(NativeTypeInstance {
                                name: "Text".into(),
                                args: Vec::new(),
                                serialized_native_type: PostgresType::Text.to_json(),
                            }),
                        ),
                    )),
                    Field::ScalarField(ScalarField::new(
                        "lastname",
                        FieldArity::Required,
                        FieldType::Scalar(
                            ScalarType::String,
                            None,
                            Some(NativeTypeInstance {
                                name: "Text".into(),
                                args: Vec::new(),
                                serialized_native_type: PostgresType::Text.to_json(),
                            }),
                        ),
                    )),
                ],
                is_generated: false,
                indices: vec![datamodel::dml::IndexDefinition {
                    name: Some("name_last_name_unique".to_string()),
                    fields: vec!["name".to_string(), "lastname".to_string()],
                    tpe: datamodel::dml::IndexType::Unique,
                }],
                id_fields: vec![],
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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

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
                            field_type: FieldType::Scalar(
                                ScalarType::Int,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                }),
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: false,
                            is_id: true,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField::new(
                            "name",
                            FieldArity::Required,
                            FieldType::Scalar(
                                ScalarType::String,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Text".into(),
                                    args: Vec::new(),
                                    serialized_native_type: PostgresType::Text.to_json(),
                                }),
                            ),
                        )),
                        Field::RelationField(RelationField::new(
                            "User",
                            FieldArity::List,
                            FieldArity::List,
                            RelationInfo {
                                to: "User".to_string(),
                                fields: vec![],
                                references: vec![],
                                name: "CityToUser".to_string(),
                                on_delete: None,
                                on_update: None,
                                legacy_referential_actions: false,
                            },
                        )),
                    ],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
                            field_type: FieldType::Scalar(
                                ScalarType::Int,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                }),
                            ),
                            database_name: None,
                            default_value: Some(DMLDefault::Expression(ValueGenerator::new_autoincrement())),
                            is_unique: false,
                            is_id: true,
                            documentation: None,
                            is_generated: false,
                            is_updated_at: false,
                            is_commented_out: false,
                            is_ignored: false,
                        }),
                        Field::ScalarField(ScalarField::new(
                            "city_id",
                            FieldArity::Required,
                            FieldType::Scalar(
                                ScalarType::Int,
                                None,
                                Some(NativeTypeInstance {
                                    name: "Integer".into(),
                                    serialized_native_type: PostgresType::Integer.to_json(),
                                    args: Vec::new(),
                                }),
                            ),
                        )),
                        Field::RelationField(RelationField {
                            name: "City".into(),
                            arity: FieldArity::Required,
                            referential_arity: FieldArity::Required,
                            documentation: None,
                            is_generated: false,
                            is_commented_out: false,
                            is_ignored: false,
                            supports_restrict_action: Some(true),
                            emulates_referential_actions: None,
                            relation_info: RelationInfo {
                                name: "CityToUser".to_string(),
                                to: "City".to_string(),
                                fields: vec!["city_id".to_string()],
                                references: vec!["id".to_string()],
                                on_delete: Some(ReferentialAction::NoAction),
                                on_update: Some(ReferentialAction::NoAction),
                                legacy_referential_actions: false,
                            },
                        }),
                    ],
                    is_generated: false,
                    indices: vec![],
                    id_fields: vec![],
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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

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
            calculate_datamodel(&schema, &Datamodel::new(), postgres_context()).expect("calculate data model");

        assert_eq!(introspection_result.data_model, ref_data_model);
    }
}

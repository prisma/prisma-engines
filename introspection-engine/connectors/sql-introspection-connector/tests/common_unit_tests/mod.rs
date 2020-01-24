use pretty_assertions::assert_eq;
use std::collections::HashSet;

use datamodel::{
    common::{ScalarType, ScalarValue},
    dml, Datamodel, DefaultValue, Field, FieldArity, FieldType, Model, OnDeleteStrategy, RelationInfo, ValueGenerator,
};
use sql_introspection_connector::calculate_datamodel::calculate_model;
use sql_schema_describer::*;

#[test]
fn a_data_model_can_be_generated_from_a_schema() {
    let col_types = &[
        ColumnTypeFamily::Int,
        ColumnTypeFamily::Float,
        ColumnTypeFamily::Boolean,
        ColumnTypeFamily::String,
        ColumnTypeFamily::DateTime,
        ColumnTypeFamily::Binary,
        ColumnTypeFamily::Json,
        ColumnTypeFamily::Uuid,
        ColumnTypeFamily::Geometric,
        ColumnTypeFamily::LogSequenceNumber,
        ColumnTypeFamily::TextSearch,
        ColumnTypeFamily::TransactionId,
    ];

    let ref_data_model = Datamodel {
        models: vec![Model {
            database_name: None,
            name: "Table1".to_string(),
            documentation: None,
            is_embedded: false,
            is_generated: false,
            indices: vec![],
            id_fields: vec![],
            fields: col_types
                .iter()
                .map(|col_type| {
                    let field_type = match col_type {
                        ColumnTypeFamily::Boolean => FieldType::Base(ScalarType::Boolean),
                        ColumnTypeFamily::DateTime => FieldType::Base(ScalarType::DateTime),
                        ColumnTypeFamily::Float => FieldType::Base(ScalarType::Float),
                        ColumnTypeFamily::Int => FieldType::Base(ScalarType::Int),
                        ColumnTypeFamily::String => FieldType::Base(ScalarType::String),
                        // XXX: We made a conscious decision to punt on mapping of ColumnTypeFamily
                        // variants that don't yet have corresponding PrismaType variants
                        _ => FieldType::Base(ScalarType::String),
                    };
                    Field {
                        name: col_type.to_string(),
                        arity: FieldArity::Optional,
                        field_type,
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    }
                })
                .collect(),
        }],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![Table {
            name: "Table1".to_string(),
            columns: col_types
                .iter()
                .map(|family| Column {
                    name: family.to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: family.to_owned(),
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                })
                .collect(),
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["primary_col".to_string()],
                sequence: None,
            }),
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#[test]
fn arity_is_preserved_when_generating_data_model_from_a_schema() {
    let ref_data_model = Datamodel {
        models: vec![Model {
            database_name: None,
            name: "Table1".to_string(),
            documentation: None,
            is_embedded: false,
            fields: vec![
                Field {
                    name: "optional".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "required".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "list".to_string(),
                    arity: FieldArity::List,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
            ],
            is_generated: false,
            indices: vec![],
            id_fields: vec![],
        }],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![Table {
            name: "Table1".to_string(),
            columns: vec![
                Column {
                    name: "optional".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "required".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "list".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::List,
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["primary_col".to_string()],
                sequence: None,
            }),
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#[test]
fn defaults_are_preserved_when_generating_data_model_from_a_schema() {
    let ref_data_model = Datamodel {
        models: vec![Model {
            database_name: None,
            name: "Table1".to_string(),
            documentation: None,
            is_embedded: false,
            fields: vec![
                Field {
                    name: "no_default".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "int_default".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: Some(dml::DefaultValue::Single(ScalarValue::Int(1))),
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "bool_default".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::Boolean),
                    database_names: Vec::new(),
                    default_value: Some(dml::DefaultValue::Single(ScalarValue::Boolean(true))),
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "float_default".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::Float),
                    database_names: Vec::new(),
                    default_value: Some(dml::DefaultValue::Single(ScalarValue::Float(1.0))),
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "string_default".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::String),
                    database_names: Vec::new(),
                    default_value: Some(dml::DefaultValue::Single(ScalarValue::String("default".to_string()))),
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
            ],
            is_generated: false,
            indices: vec![],
            id_fields: vec![],
        }],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![Table {
            name: "Table1".to_string(),
            columns: vec![
                Column {
                    name: "no_default".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "int_default".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: Some("'1'".to_string()),
                    auto_increment: false,
                },
                Column {
                    name: "bool_default".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Boolean,
                        arity: ColumnArity::Nullable,
                    },
                    default: Some("'1'".to_string()),
                    auto_increment: false,
                },
                Column {
                    name: "float_default".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Float,
                        arity: ColumnArity::Nullable,
                    },
                    default: Some("'1.0'".to_string()),
                    auto_increment: false,
                },
                Column {
                    name: "string_default".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::String,
                        arity: ColumnArity::Nullable,
                    },
                    default: Some("default".to_string()),
                    auto_increment: false,
                },
            ],
            indices: vec![],
            primary_key: None,
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
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
                fields: vec![Field {
                    name: "primary".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: Some(DefaultValue::Expression(ValueGenerator::new_autoincrement_bang())),
                    is_unique: false,
                    is_id: true,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                }],
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
                fields: vec![Field {
                    name: "primary".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: true,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                }],
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
                fields: vec![Field {
                    name: "primary".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: Some(DefaultValue::Expression(ValueGenerator::new_autoincrement_bang())),
                    is_unique: false,
                    is_id: true,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                }],
                is_generated: false,
                indices: vec![],
                id_fields: vec![],
            },
        ],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![
            Table {
                name: "Table1".to_string(),
                columns: vec![Column {
                    name: "primary".to_string(),
                    tpe: ColumnType {
                        raw: "integer".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },
                    default: None,
                    auto_increment: true,
                }],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["primary".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![],
            },
            Table {
                name: "Table2".to_string(),
                columns: vec![Column {
                    name: "primary".to_string(),
                    tpe: ColumnType {
                        raw: "integer".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },
                    default: None,
                    auto_increment: false,
                }],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["primary".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![],
            },
            Table {
                name: "Table3".to_string(),
                columns: vec![Column {
                    name: "primary".to_string(),
                    tpe: ColumnType {
                        raw: "integer".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },
                    default: None,
                    auto_increment: true,
                }],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["primary".to_string()],
                    sequence: Some(Sequence {
                        name: "sequence".to_string(),
                        initial_value: 1,
                        allocation_size: 1,
                    }),
                }),
                foreign_keys: vec![],
            },
        ],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#[test]
fn uniqueness_is_preserved_when_generating_data_model_from_a_schema() {
    let ref_data_model = Datamodel {
        models: vec![Model {
            database_name: None,
            name: "Table1".to_string(),
            documentation: None,
            is_embedded: false,
            fields: vec![
                Field {
                    name: "non_unique".to_string(),
                    arity: FieldArity::Optional,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "unique".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: true,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
            ],
            is_generated: false,
            indices: vec![],
            id_fields: vec![],
        }],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![Table {
            name: "Table1".to_string(),
            columns: vec![
                Column {
                    name: "non_unique".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "unique".to_string(),
                    tpe: ColumnType {
                        raw: "raw".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },
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
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#[test]
#[ignore]
fn compound_foreign_keys_are_preserved_when_generating_data_model_from_a_schema() {
    let ref_data_model = Datamodel {
        models: vec![
            Model {
                database_name: None,
                name: "City".to_string(),
                documentation: None,
                is_embedded: false,
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::Int),
                        database_names: Vec::new(),
                        default_value: Some(DefaultValue::Expression(ValueGenerator::new_autoincrement_bang())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                    Field {
                        name: "name".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::String),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
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
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::Int),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                    Field {
                        name: "city-id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Relation(RelationInfo {
                            name: "".to_string(),
                            to: "City".to_string(),
                            to_fields: vec!["id".to_string()],
                            on_delete: OnDeleteStrategy::None,
                        }),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                    Field {
                        name: "city-name".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Relation(RelationInfo {
                            name: "".to_string(),
                            to: "City".to_string(),
                            to_fields: vec!["name".to_string()],
                            on_delete: OnDeleteStrategy::None,
                        }),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                ],
                is_generated: false,
                indices: vec![],
                id_fields: vec![],
            },
        ],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![
            Table {
                name: "City".to_string(),
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "name".to_string(),
                        tpe: ColumnType {
                            raw: "text".to_string(),
                            family: ColumnTypeFamily::String,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["id".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![],
            },
            Table {
                name: "User".to_string(),
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "city-id".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: false,
                    },
                    Column {
                        name: "city-name".to_string(),
                        tpe: ColumnType {
                            raw: "text".to_string(),
                            family: ColumnTypeFamily::String,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![],
                primary_key: None,
                foreign_keys: vec![ForeignKey {
                    // what does this mean? the from columns are not targeting a specific to column?
                    constraint_name: None,
                    columns: vec!["city-id".to_string(), "city-name".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::NoAction,
                    referenced_columns: vec!["id".to_string(), "name".to_string()],
                }],
            },
        ],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#[test]
fn multi_field_uniques_are_preserved_when_generating_data_model_from_a_schema() {
    let ref_data_model = Datamodel {
        models: vec![Model {
            database_name: None,
            name: "User".to_string(),
            documentation: None,
            is_embedded: false,
            fields: vec![
                Field {
                    name: "id".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::Int),
                    database_names: Vec::new(),
                    default_value: Some(DefaultValue::Expression(ValueGenerator::new_autoincrement_bang())),
                    is_unique: false,
                    is_id: true,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "name".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::String),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
                Field {
                    name: "lastname".to_string(),
                    arity: FieldArity::Required,
                    field_type: FieldType::Base(ScalarType::String),
                    database_names: Vec::new(),
                    default_value: None,
                    is_unique: false,
                    is_id: false,
                    documentation: None,
                    is_generated: false,
                    is_updated_at: false,
                },
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
        tables: vec![Table {
            name: "User".to_string(),
            columns: vec![
                Column {
                    name: "id".to_string(),
                    tpe: ColumnType {
                        raw: "integer".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },
                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "name".to_string(),
                    tpe: ColumnType {
                        raw: "text".to_string(),
                        family: ColumnTypeFamily::String,
                        arity: ColumnArity::Required,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "lastname".to_string(),
                    tpe: ColumnType {
                        raw: "text".to_string(),
                        family: ColumnTypeFamily::String,
                        arity: ColumnArity::Required,
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
            }),
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
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
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::Int),
                        database_names: Vec::new(),
                        default_value: Some(DefaultValue::Expression(ValueGenerator::new_autoincrement_bang())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                    Field {
                        name: "name".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::String),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                    Field {
                        name: "users".to_string(),
                        arity: FieldArity::List,
                        field_type: FieldType::Relation(RelationInfo {
                            to: "User".to_string(),
                            to_fields: vec!["city_id".to_string()],
                            name: "CityToUser".to_string(),
                            on_delete: OnDeleteStrategy::None,
                        }),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
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
                fields: vec![
                    Field {
                        name: "id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Base(ScalarType::Int),
                        database_names: Vec::new(),
                        default_value: Some(DefaultValue::Expression(ValueGenerator::new_autoincrement_bang())),
                        is_unique: false,
                        is_id: true,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                    Field {
                        name: "city_id".to_string(),
                        arity: FieldArity::Required,
                        field_type: FieldType::Relation(RelationInfo {
                            name: "CityToUser".to_string(),
                            to: "City".to_string(),
                            to_fields: vec!["id".to_string()],
                            on_delete: OnDeleteStrategy::None,
                        }),
                        database_names: Vec::new(),
                        default_value: None,
                        is_unique: false,
                        is_id: false,
                        documentation: None,
                        is_generated: false,
                        is_updated_at: false,
                    },
                ],
                is_generated: false,
                indices: vec![],
                id_fields: vec![],
            },
        ],
        enums: vec![],
    };

    let schema = SqlSchema {
        tables: vec![
            Table {
                name: "City".to_string(),
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "name".to_string(),
                        tpe: ColumnType {
                            raw: "text".to_string(),
                            family: ColumnTypeFamily::String,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["id".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![],
            },
            Table {
                name: "User".to_string(),
                columns: vec![
                    Column {
                        name: "id".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "city_id".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![],
                primary_key: Some(PrimaryKey {
                    columns: vec!["id".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![ForeignKey {
                    constraint_name: None,
                    columns: vec!["city_id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::NoAction,
                    referenced_columns: vec!["id".to_string()],
                }],
            },
        ],
        enums: vec![],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#[test]
fn enums_are_preserved_when_generating_data_model_from_a_schema() {
    let ref_data_model = Datamodel {
        models: vec![],
        enums: vec![dml::Enum {
            name: "Enum".to_string(),
            database_name: None,
            documentation: None,
            values: vec!["a".to_string(), "b".to_string()],
        }],
    };

    let mut enum_values = HashSet::new();
    enum_values.insert("a".to_string());
    enum_values.insert("b".to_string());
    let schema = SqlSchema {
        tables: vec![],
        enums: vec![Enum {
            name: "Enum".to_string(),
            values: enum_values,
        }],
        sequences: vec![],
    };
    let data_model = calculate_model(&schema).expect("calculate data model");

    assert_eq!(data_model, ref_data_model);
}

#![allow(non_snake_case)]
#![allow(unused)]

use barrel::{types, Migration};
use log::{debug, LevelFilter};
use pretty_assertions::assert_eq;
use quaint::connector::{Queryable, Sqlite as SqliteDatabaseClient};
use sql_schema_describer::*;
use std::collections::HashSet;
use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::{thread, time};

const SCHEMA: &str = "DatabaseInspectorTest";

#[test]
fn database_schema_is_serializable() {
    let mut enum_values = vec!["option1".to_string(), "option2".to_string()];
    let schema = SqlSchema {
        tables: vec![
            Table {
                name: "table1".to_string(),
                columns: vec![
                    Column {
                        name: "column1".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: true,
                    },
                    Column {
                        name: "column2".to_string(),
                        tpe: ColumnType {
                            raw: "varchar(255)".to_string(),
                            family: ColumnTypeFamily::String,
                            arity: ColumnArity::Nullable,
                        },
                        default: Some(DefaultValue::VALUE("default value".to_string())),
                        auto_increment: false,
                    },
                    Column {
                        name: "column3".to_string(),
                        tpe: ColumnType {
                            raw: "integer".to_string(),
                            family: ColumnTypeFamily::Int,
                            arity: ColumnArity::Required,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ],
                indices: vec![Index {
                    name: "column2".to_string(),
                    columns: vec!["column2".to_string()],
                    tpe: IndexType::Normal,
                }],
                primary_key: Some(PrimaryKey {
                    columns: vec!["column1".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![ForeignKey {
                    constraint_name: None,
                    columns: vec!["column3".to_string()],
                    referenced_table: "table2".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::NoAction,
                }],
            },
            Table {
                name: "table2".to_string(),
                columns: vec![Column {
                    name: "id".to_string(),
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
                    columns: vec!["id".to_string()],
                    sequence: None,
                }),
                foreign_keys: vec![],
            },
        ],
        enums: vec![Enum {
            name: "enum1".to_string(),
            values: enum_values,
        }],
        sequences: vec![Sequence {
            name: "sequence1".to_string(),
            initial_value: 1,
            allocation_size: 32,
        }],
    };
    let ref_schema_json = include_str!("./resources/schema.json");
    let ref_schema: SqlSchema = serde_json::from_str(ref_schema_json).expect("deserialize reference schema");

    let schema_json = serde_json::to_string(&schema).expect("serialize schema to JSON");
    let schema_deser: SqlSchema = serde_json::from_str(&schema_json).expect("deserialize schema");

    // Verify that deserialized schema is equivalent
    assert_eq!(schema_deser, schema);
    // Verify that schema deserialized from reference JSON is equivalent
    assert_eq!(ref_schema, schema);
}

#[test]
fn database_schema_without_primary_key_is_serializable() {
    let schema = SqlSchema {
        tables: vec![Table {
            name: "table1".to_string(),
            columns: vec![Column {
                name: "column1".to_string(),
                tpe: ColumnType {
                    raw: "integer".to_string(),
                    family: ColumnTypeFamily::Int,
                    arity: ColumnArity::Nullable,
                },
                default: None,
                auto_increment: false,
            }],
            indices: vec![],
            primary_key: None,
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let ref_schema_json = include_str!("./resources/schema-without-primary-key.json");
    let ref_schema: SqlSchema = serde_json::from_str(ref_schema_json).expect("deserialize reference schema");

    let schema_json = serde_json::to_string(&schema).expect("serialize schema to JSON");
    let schema_deser: SqlSchema = serde_json::from_str(&schema_json).expect("deserialize schema");

    // Verify that deserialized schema is equivalent
    assert_eq!(schema_deser, schema);
    // Verify that schema deserialized from reference JSON is equivalent
    assert_eq!(ref_schema, schema);
}

#[test]
fn database_schema_is_serializable_for_every_column_type_family() {
    // Add a column of every column type family
    let mut columns: Vec<Column> = vec![
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
    ]
    .iter()
    .enumerate()
    .map(|(i, family)| Column {
        name: format!("column{}", i + 1),
        tpe: ColumnType {
            raw: "raw type".to_string(),
            family: family.to_owned(),
            arity: ColumnArity::Nullable,
        },
        default: None,
        auto_increment: false,
    })
    .collect();
    let schema = SqlSchema {
        tables: vec![Table {
            name: "table1".to_string(),
            columns,
            indices: vec![],
            primary_key: None,
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let ref_schema_json = include_str!("./resources/schema-all-column-type-families.json");
    let ref_schema: SqlSchema = serde_json::from_str(ref_schema_json).expect("deserialize reference schema");

    let schema_json = serde_json::to_string(&schema).expect("serialize schema to JSON");
    let schema_deser: SqlSchema = serde_json::from_str(&schema_json).expect("deserialize schema");

    // Verify that deserialized schema is equivalent
    assert_eq!(schema_deser, schema);
    // Verify that schema deserialized from reference JSON is equivalent
    assert_eq!(ref_schema, schema);
}

#[test]
fn database_schema_is_serializable_for_every_column_arity() {
    // Add a column of every arity
    let mut columns: Vec<Column> = vec![ColumnArity::Required, ColumnArity::Nullable, ColumnArity::List]
        .iter()
        .enumerate()
        .map(|(i, arity)| Column {
            name: format!("column{}", i + 1),
            tpe: ColumnType {
                raw: "int".to_string(),
                family: ColumnTypeFamily::Int,
                arity: arity.to_owned(),
            },
            default: None,
            auto_increment: false,
        })
        .collect();
    let schema = SqlSchema {
        tables: vec![Table {
            name: "table1".to_string(),
            columns,
            indices: vec![],
            primary_key: None,
            foreign_keys: vec![],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let ref_schema_json = include_str!("./resources/schema-all-column-arities.json");
    let ref_schema: SqlSchema = serde_json::from_str(ref_schema_json).expect("deserialize reference schema");

    let schema_json = serde_json::to_string(&schema).expect("serialize schema to JSON");
    let schema_deser: SqlSchema = serde_json::from_str(&schema_json).expect("deserialize schema");

    // Verify that deserialized schema is equivalent
    assert_eq!(schema_deser, schema);
    // Verify that schema deserialized from reference JSON is equivalent
    assert_eq!(ref_schema, schema);
}

#[test]
fn database_schema_is_serializable_for_every_foreign_key_action() {
    // Add a foreign key of every possible action
    let schema = SqlSchema {
        tables: vec![Table {
            name: "table1".to_string(),
            columns: vec![
                Column {
                    name: "column1".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    auto_increment: false,
                    default: None,
                },
                Column {
                    name: "column2".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    auto_increment: false,
                    default: None,
                },
                Column {
                    name: "column3".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    auto_increment: false,
                    default: None,
                },
                Column {
                    name: "column4".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    auto_increment: false,
                    default: None,
                },
                Column {
                    name: "column5".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    auto_increment: false,
                    default: None,
                },
            ],
            indices: vec![],
            primary_key: None,
            foreign_keys: vec![
                ForeignKey {
                    constraint_name: None,
                    columns: vec!["column1".to_string()],
                    referenced_table: "table2".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::NoAction,
                },
                ForeignKey {
                    constraint_name: None,
                    columns: vec!["column2".to_string()],
                    referenced_table: "table2".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::Restrict,
                },
                ForeignKey {
                    constraint_name: None,
                    columns: vec!["column3".to_string()],
                    referenced_table: "table2".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::Cascade,
                },
                ForeignKey {
                    constraint_name: None,
                    columns: vec!["column4".to_string()],
                    referenced_table: "table2".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::SetNull,
                },
                ForeignKey {
                    constraint_name: None,
                    columns: vec!["column5".to_string()],
                    referenced_table: "table2".to_string(),
                    referenced_columns: vec!["id".to_string()],
                    on_delete_action: ForeignKeyAction::SetDefault,
                },
            ],
        }],
        enums: vec![],
        sequences: vec![],
    };
    let ref_schema_json = include_str!("./resources/schema-all-foreign-key-actions.json");
    let ref_schema: SqlSchema = serde_json::from_str(ref_schema_json).expect("deserialize reference schema");

    let schema_json = serde_json::to_string(&schema).expect("serialize schema to JSON");
    let schema_deser: SqlSchema = serde_json::from_str(&schema_json).expect("deserialize schema");

    // Verify that deserialized schema is equivalent
    assert_eq!(schema_deser, schema);
    // Verify that schema deserialized from reference JSON is equivalent
    assert_eq!(ref_schema, schema);
}

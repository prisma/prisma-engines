use barrel::{types, Migration};
use database_introspection::*;
use pretty_assertions::assert_eq;

mod common;
mod sqlite;

use common::*;
use sqlite::*;

#[test]
fn sqlite_column_types_must_work() {
    setup();

    let mut migration = Migration::new().schema(SCHEMA);
    migration.create_table("User", move |t| {
        t.add_column("int4_col", types::integer());
        t.add_column("text_col", types::text());
        t.add_column("real_col", types::float());
        t.add_column("primary_col", types::primary());
    });

    let full_sql = migration.make::<barrel::backend::Sqlite>();
    let inspector = get_sqlite_connector(&full_sql);
    let result = inspector.introspect(SCHEMA).expect("introspection");
    let table = result.get_table("User").expect("couldn't get User table");
    let mut expected_columns = vec![
        Column {
            name: "int4_col".to_string(),
            tpe: ColumnType {
                raw: "INTEGER".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "text_col".to_string(),
            tpe: ColumnType {
                raw: "TEXT".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "real_col".to_string(),
            tpe: ColumnType {
                raw: "REAL".to_string(),
                family: ColumnTypeFamily::Float,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "primary_col".to_string(),
            tpe: ColumnType {
                raw: "INTEGER".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: true,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!(
        table,
        &Table {
            name: "User".to_string(),
            columns: expected_columns,
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["primary_col".to_string()],
                sequence: None,
            }),
            foreign_keys: vec![],
        }
    );
}

#[test]
fn sqlite_foreign_key_on_delete_must_be_handled() {
    setup();

    let sql = format!(
        "CREATE TABLE \"{0}\".City (id INTEGER NOT NULL PRIMARY KEY);
         CREATE TABLE \"{0}\".User (
            id INTEGER NOT NULL PRIMARY KEY,
            city INTEGER REFERENCES City(id) ON DELETE NO ACTION,
            city_cascade INTEGER REFERENCES City(id) ON DELETE CASCADE,
            city_restrict INTEGER REFERENCES City (id) ON DELETE RESTRICT,
            city_set_default INTEGER REFERENCES City(id) ON DELETE SET DEFAULT,
            city_set_null INTEGER REFERENCES City(id) ON DELETE SET NULL
        )",
        SCHEMA
    );
    let inspector = get_sqlite_connector(&sql);

    let schema = inspector.introspect(SCHEMA).expect("introspection");
    let mut table = schema.get_table("User").expect("get User table").to_owned();
    table.foreign_keys.sort_unstable_by_key(|fk| fk.columns.clone());

    assert_eq!(
        table,
        Table {
            name: "User".to_string(),
            columns: vec![
                Column {
                    name: "city".to_string(),
                    tpe: ColumnType {
                        raw: "INTEGER".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_cascade".to_string(),
                    tpe: ColumnType {
                        raw: "INTEGER".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_restrict".to_string(),
                    tpe: ColumnType {
                        raw: "INTEGER".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_set_default".to_string(),
                    tpe: ColumnType {
                        raw: "INTEGER".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_set_null".to_string(),
                    tpe: ColumnType {
                        raw: "INTEGER".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "id".to_string(),
                    tpe: ColumnType {
                        raw: "INTEGER".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: true,
                },
            ],
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["id".to_string()],
                sequence: None,
            }),
            foreign_keys: vec![
                ForeignKey {
                    columns: vec!["city".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::NoAction,
                },
                ForeignKey {
                    columns: vec!["city_cascade".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::Cascade,
                },
                ForeignKey {
                    columns: vec!["city_restrict".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::Restrict,
                },
                ForeignKey {
                    columns: vec!["city_set_default".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::SetDefault,
                },
                ForeignKey {
                    columns: vec!["city_set_null".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::SetNull,
                },
            ],
        }
    );
}

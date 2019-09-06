use barrel::{types, Migration};
use pretty_assertions::assert_eq;
use sql_schema_describer::{IntrospectionConnector, *};

mod common;
mod mysql;
mod postgres;
mod sqlite;

use crate::common::*;
use crate::mysql::*;
use crate::postgres::*;
use crate::sqlite::*;

#[derive(Debug, PartialEq, Copy, Clone)]
enum DbType {
    Postgres,
    MySql,
    Sqlite,
}

fn int_type(db_type: DbType) -> String {
    match db_type {
        DbType::Postgres => "int4".to_string(),
        DbType::Sqlite => "INTEGER".to_string(),
        DbType::MySql => "int".to_string(),
    }
}

fn varchar_type(db_type: DbType, length: u64) -> String {
    match db_type {
        DbType::Postgres => "varchar".to_string(),
        DbType::MySql => "varchar".to_string(),
        DbType::Sqlite => format!("VARCHAR({})", length),
    }
}

#[test]
fn is_required_must_work() {
    setup();

    test_each_backend(
        |_, migration| {
            migration.create_table("User", |t| {
                t.add_column("column1", types::integer().nullable(false));
                t.add_column("column2", types::integer().nullable(true));
            });
        },
        |db_type, inspector| {
            let result = inspector.introspect(SCHEMA).expect("introspecting");
            let user_table = result.get_table("User").expect("getting User table");
            let expected_columns = vec![
                Column {
                    name: "column1".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "column2".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
            ];
            assert_eq!(user_table.columns, expected_columns);
        },
    );
}

#[test]
fn foreign_keys_must_work() {
    setup();

    test_each_backend(
        |db_type, migration| {
            migration.create_table("City", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("User", move |t| {
                // barrel does not render foreign keys correctly for mysql
                // TODO: Investigate
                if db_type == DbType::MySql {
                    t.add_column("city", types::integer());
                    t.inject_custom("FOREIGN KEY(city) REFERENCES City(id)");
                } else {
                    t.add_column("city", types::foreign("City", "id"));
                }
            });
        },
        |db_type, inspector| {
            let schema = inspector.introspect(SCHEMA).expect("introspection");
            let user_table = schema.get_table("User").expect("couldn't get User table");
            let expected_columns = vec![Column {
                name: "city".to_string(),
                tpe: ColumnType {
                    raw: int_type(db_type),
                    family: ColumnTypeFamily::Int,
                },
                arity: ColumnArity::Required,
                default: None,
                auto_increment: false,
            }];

            let on_delete_action = match db_type {
                DbType::MySql => ForeignKeyAction::Restrict,
                _ => ForeignKeyAction::NoAction,
            };
            assert_eq!(
                user_table,
                &Table {
                    name: "User".to_string(),
                    columns: expected_columns,
                    indices: vec![],
                    primary_key: None,
                    foreign_keys: vec![ForeignKey {
                        columns: vec!["city".to_string()],
                        referenced_columns: vec!["id".to_string()],
                        referenced_table: "City".to_string(),
                        on_delete_action,
                    }],
                }
            );
        },
    );
}

#[test]
fn multi_column_foreign_keys_must_work() {
    setup();

    test_each_backend(
        |db_type, migration| {
            migration.create_table("City", move |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::varchar(255));
                if db_type != DbType::Sqlite {
                    t.inject_custom("constraint uniq unique (id, name)");
                }
            });
            migration.create_table("User", move |t| {
                t.add_column("city", types::integer());
                t.add_column("city_name", types::varchar(255));
                if db_type == DbType::MySql {
                    t.inject_custom("FOREIGN KEY(city, city_name) REFERENCES City(id, name)");
                } else {
                    let relation_prefix = match db_type {
                        DbType::Postgres => format!("\"{}\".", SCHEMA),
                        _ => "".to_string(),
                    };
                    t.inject_custom(format!(
                        "FOREIGN KEY(city, city_name) REFERENCES {}\"City\"(id, name)",
                        relation_prefix
                    ));
                }
            });
        },
        |db_type, inspector| {
            let schema = inspector.introspect(SCHEMA).expect("introspection");
            let user_table = schema.get_table("User").expect("couldn't get User table");
            let expected_columns = vec![
                Column {
                    name: "city".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_name".to_string(),
                    tpe: ColumnType {
                        raw: varchar_type(db_type, 255),
                        family: ColumnTypeFamily::String,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
            ];

            let on_delete_action = match db_type {
                DbType::MySql => ForeignKeyAction::Restrict,
                _ => ForeignKeyAction::NoAction,
            };
            assert_eq!(
                user_table,
                &Table {
                    name: "User".to_string(),
                    columns: expected_columns,
                    indices: vec![],
                    primary_key: None,
                    foreign_keys: vec![ForeignKey {
                        columns: vec!["city".to_string(), "city_name".to_string()],
                        referenced_columns: vec!["id".to_string(), "name".to_string()],
                        referenced_table: "City".to_string(),
                        on_delete_action,
                    },],
                }
            );
        },
    );
}

#[test]
fn names_with_hyphens_must_work() {
    setup();

    test_each_backend(
        |_, migration| {
            migration.create_table("User-table", |t| {
                t.add_column("column-1", types::integer().nullable(false));
            });
        },
        |db_type, inspector| {
            let result = inspector.introspect(SCHEMA).expect("introspecting");
            let user_table = result.get_table("User-table").expect("getting User table");
            let expected_columns = vec![Column {
                name: "column-1".to_string(),
                tpe: ColumnType {
                    raw: int_type(db_type),
                    family: ColumnTypeFamily::Int,
                },
                arity: ColumnArity::Required,
                default: None,
                auto_increment: false,
            }];
            assert_eq!(user_table.columns, expected_columns);
        },
    );
}

#[test]
fn composite_primary_keys_must_work() {
    setup();

    test_each_backend(
        |db_type, migration| {
            let sql = match db_type {
                DbType::MySql => format!(
                    "CREATE TABLE `{0}`.`User` (
                        id INTEGER NOT NULL,
                        name VARCHAR(255) NOT NULL,
                        PRIMARY KEY(id, name)
                    )",
                    SCHEMA
                ),
                _ => format!(
                    "CREATE TABLE \"{0}\".\"User\" (
                        id INTEGER NOT NULL,
                        name VARCHAR(255) NOT NULL,
                        PRIMARY KEY(id, name)
                    )",
                    SCHEMA
                ),
            };
            migration.inject_custom(&sql);
        },
        |db_type, inspector| {
            let schema = inspector.introspect(SCHEMA).expect("introspection");
            let table = schema.get_table("User").expect("couldn't get User table");
            let (exp_int, exp_varchar) = match db_type {
                DbType::Sqlite => ("INTEGER", "VARCHAR(255)"),
                DbType::MySql => ("int", "varchar"),
                DbType::Postgres => ("int4", "varchar"),
            };
            let expected_indices = match db_type {
                DbType::Sqlite => vec![Index {
                    name: "sqlite_autoindex_User_1".to_string(),
                    columns: vec!["id".to_string(), "name".to_string()],
                    tpe: IndexType::Unique,
                }],
                _ => vec![],
            };
            let mut expected_columns = vec![
                Column {
                    name: "id".to_string(),
                    tpe: ColumnType {
                        raw: exp_int.to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "name".to_string(),
                    tpe: ColumnType {
                        raw: exp_varchar.to_string(),
                        family: ColumnTypeFamily::String,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
            ];
            expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

            assert_eq!(
                table,
                &Table {
                    name: "User".to_string(),
                    columns: expected_columns,
                    indices: expected_indices,
                    primary_key: Some(PrimaryKey {
                        columns: vec!["id".to_string(), "name".to_string()],
                        sequence: None,
                    }),
                    foreign_keys: vec![],
                }
            );
        },
    );
}

#[test]
fn indices_must_work() {
    setup();

    test_each_backend(
        |_, migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("count", types::integer());
                t.add_index("count", types::index(vec!["count"]));
            });
        },
        |db_type, inspector| {
            let result = inspector.introspect(&SCHEMA.to_string()).expect("introspecting");
            let user_table = result.get_table("User").expect("getting User table");
            let default = match db_type {
                DbType::Postgres => Some(format!("nextval('\"{}\".\"User_id_seq\"'::regclass)", SCHEMA)),
                _ => None,
            };
            let expected_columns = vec![
                Column {
                    name: "count".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "id".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default,
                    auto_increment: true,
                },
            ];
            let pk_sequence = match db_type {
                DbType::Postgres => Some(Sequence {
                    name: "User_id_seq".to_string(),
                    allocation_size: 1,
                    initial_value: 1,
                }),
                _ => None,
            };
            assert_eq!(
                user_table,
                &Table {
                    name: "User".to_string(),
                    columns: expected_columns,
                    indices: vec![Index {
                        name: "count".to_string(),
                        columns: vec!["count".to_string()],
                        tpe: IndexType::Normal,
                    },],
                    primary_key: Some(PrimaryKey {
                        columns: vec!["id".to_string()],
                        sequence: pk_sequence,
                    }),
                    foreign_keys: vec![],
                }
            );
        },
    );
}

#[test]
fn column_uniqueness_must_be_detected() {
    setup();

    test_each_backend(
        |db_type, migration| {
            migration.create_table("User", move |t| {
                t.add_column("uniq1", types::integer().unique(true));
                t.add_column("uniq2", types::integer());
            });
            let index_sql = match db_type {
                DbType::MySql => format!("CREATE UNIQUE INDEX `uniq` ON `{}`.`User` (uniq2)", SCHEMA),
                DbType::Sqlite => format!("CREATE UNIQUE INDEX \"{}\".\"uniq\" ON \"User\" (uniq2)", SCHEMA),
                DbType::Postgres => format!("CREATE UNIQUE INDEX \"uniq\" ON \"{}\".\"User\" (uniq2)", SCHEMA),
            };
            migration.inject_custom(index_sql);
        },
        |db_type, inspector| {
            let result = inspector.introspect(&SCHEMA.to_string()).expect("introspecting");
            let user_table = result.get_table("User").expect("getting User table");
            let expected_columns = vec![
                Column {
                    name: "uniq1".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "uniq2".to_string(),
                    tpe: ColumnType {
                        raw: int_type(db_type),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Required,
                    default: None,
                    auto_increment: false,
                },
            ];
            let mut expected_indices = vec![Index {
                name: "uniq".to_string(),
                columns: vec!["uniq2".to_string()],
                tpe: IndexType::Unique,
            }];
            match db_type {
                DbType::MySql => expected_indices.insert(
                    0,
                    Index {
                        name: "uniq1".to_string(),
                        columns: vec!["uniq1".to_string()],
                        tpe: IndexType::Unique,
                    },
                ),
                DbType::Postgres => expected_indices.insert(
                    0,
                    Index {
                        name: "User_uniq1_key".to_string(),
                        columns: vec!["uniq1".to_string()],
                        tpe: IndexType::Unique,
                    },
                ),
                DbType::Sqlite => expected_indices.push(Index {
                    name: "sqlite_autoindex_User_1".to_string(),
                    columns: vec!["uniq1".to_string()],
                    tpe: IndexType::Unique,
                }),
            };
            assert_eq!(
                user_table,
                &Table {
                    name: "User".to_string(),
                    columns: expected_columns,
                    indices: expected_indices,
                    primary_key: None,
                    foreign_keys: vec![],
                }
            );
            assert!(
                user_table.is_column_unique(&user_table.columns[0]),
                "Column 1 should return true for is_unique"
            );
            assert!(
                user_table.is_column_unique(&user_table.columns[1]),
                "Column 2 should return true for is_unique"
            );
        },
    );
}

#[test]
fn defaults_must_work() {
    setup();

    test_each_backend(
        |_, migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().default(1).nullable(true));
            });
        },
        |db_type, inspector| {
            let result = inspector.introspect(&SCHEMA.to_string()).expect("introspecting");
            let user_table = result.get_table("User").expect("getting User table");
            let default = match db_type {
                DbType::Sqlite => "'1'".to_string(),
                _ => "1".to_string(),
            };
            let expected_columns = vec![Column {
                name: "id".to_string(),
                tpe: ColumnType {
                    raw: int_type(db_type),
                    family: ColumnTypeFamily::Int,
                },
                arity: ColumnArity::Nullable,
                default: Some(default),
                auto_increment: false,
            }];
            assert_eq!(
                user_table,
                &Table {
                    name: "User".to_string(),
                    columns: expected_columns,
                    indices: vec![],
                    primary_key: None,
                    foreign_keys: vec![],
                }
            );
        },
    );
}

fn test_each_backend<MigrationFn, TestFn>(mut migration_fn: MigrationFn, test_fn: TestFn)
where
    MigrationFn: FnMut(DbType, &mut Migration) -> (),
    TestFn: Fn(DbType, &mut dyn IntrospectionConnector) -> (),
{
    // SQLite
    {
        let mut migration = Migration::new().schema(SCHEMA);
        migration_fn(DbType::Sqlite, &mut migration);
        let full_sql = migration.make::<barrel::backend::Sqlite>();
        let mut inspector = get_sqlite_connector(&full_sql);

        test_fn(DbType::Sqlite, &mut inspector);
    }
    // Postgres
    {
        let mut migration = Migration::new().schema(SCHEMA);
        migration_fn(DbType::Postgres, &mut migration);
        let full_sql = migration.make::<barrel::backend::Pg>();
        let mut inspector = get_postgres_connector(&full_sql);

        test_fn(DbType::Postgres, &mut inspector);
    }
    // MySQL
    {
        let mut migration = Migration::new().schema(SCHEMA);
        migration_fn(DbType::MySql, &mut migration);
        let full_sql = migration.make::<barrel::backend::MySql>();
        let mut inspector = get_mysql_connector(&full_sql);

        test_fn(DbType::MySql, &mut inspector);
    }
}

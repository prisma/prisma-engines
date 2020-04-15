use barrel::types;
use pretty_assertions::assert_eq;
use quaint::prelude::SqlFamily;
use sql_schema_describer::*;
use test_macros::test_each_connector;

mod common;
mod mysql;
mod postgres;
mod sqlite;
mod test_api;

use crate::common::*;
use crate::test_api::*;
use prisma_value::PrismaValue;

fn int_type(db_type: SqlFamily) -> String {
    match db_type {
        SqlFamily::Postgres => "int4".to_string(),
        SqlFamily::Sqlite => "INTEGER".to_string(),
        SqlFamily::Mysql => "int".to_string(),
    }
}

fn varchar_type(db_type: SqlFamily, length: u64) -> String {
    match db_type {
        SqlFamily::Postgres => "varchar".to_string(),
        SqlFamily::Mysql => "varchar".to_string(),
        SqlFamily::Sqlite => format!("VARCHAR({})", length),
    }
}

#[test_each_connector]
async fn is_required_must_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", |t| {
                t.add_column("column1", types::integer().nullable(false));
                t.add_column("column2", types::integer().nullable(true));
            });
        })
        .await;

    let result = api.describe().await.expect("describing");
    let user_table = result.get_table("User").expect("getting User table");
    let expected_columns = vec![
        Column {
            name: "column1".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "column2".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Nullable,
            },
            default: None,
            auto_increment: false,
        },
    ];

    assert_eq!(user_table.columns, expected_columns);
}

#[test_each_connector]
async fn foreign_keys_must_work(api: &TestApi) {
    let sql_family = api.sql_family();

    api.barrel()
        .execute(|migration| {
            migration.create_table("City", |t| {
                t.add_column("id", types::primary());
            });
            migration.create_table("User", move |t| {
                // barrel does not render foreign keys correctly for mysql
                // TODO: Investigate
                if sql_family == SqlFamily::Mysql {
                    t.add_column("city", types::integer());
                    t.inject_custom("FOREIGN KEY(city) REFERENCES City(id) ON DELETE RESTRICT");
                } else {
                    t.add_column("city", types::foreign("City", "id"));
                }
            });
        })
        .await;

    let schema = api.describe().await.expect("describe failed");
    let user_table = schema.get_table("User").expect("couldn't get User table");
    let expected_columns = vec![Column {
        name: "city".to_string(),
        tpe: ColumnType {
            raw: int_type(api.sql_family()),
            family: ColumnTypeFamily::Int,
            arity: ColumnArity::Required,
        },
        default: None,
        auto_increment: false,
    }];

    let on_delete_action = match api.sql_family() {
        SqlFamily::Mysql => ForeignKeyAction::Restrict,
        _ => ForeignKeyAction::NoAction,
    };
    let expected_indexes = if sql_family.is_mysql() {
        vec![Index {
            name: "city".to_owned(),
            columns: vec!["city".to_owned()],
            tpe: IndexType::Normal,
        }]
    } else {
        vec![]
    };

    assert_eq!(
        user_table,
        &Table {
            name: "User".to_string(),
            columns: expected_columns,
            indices: expected_indexes,
            primary_key: None,
            foreign_keys: vec![ForeignKey {
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("User_city_fkey".to_owned()),
                    SqlFamily::Mysql => Some("User_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                },
                columns: vec!["city".to_string()],
                referenced_columns: vec!["id".to_string()],
                referenced_table: "City".to_string(),
                on_delete_action,
            }],
        }
    );
}

#[test_each_connector]
async fn multi_column_foreign_keys_must_work(api: &TestApi) {
    let sql_family = api.sql_family();
    let schema = api.schema_name().to_owned();

    api.barrel()
        .execute(|migration| {
            migration.create_table("City", move |t| {
                t.add_column("id", types::primary());
                t.add_column("name", types::varchar(255));
                if sql_family != SqlFamily::Sqlite {
                    t.inject_custom("constraint uniq unique (name, id)");
                }
            });
            migration.create_table("User", move |t| {
                t.add_column("city", types::integer());
                t.add_column("city_name", types::varchar(255));
                if sql_family == SqlFamily::Mysql {
                    t.inject_custom("FOREIGN KEY(city_name, city) REFERENCES City(name, id) ON DELETE RESTRICT");
                } else {
                    let relation_prefix = match sql_family {
                        SqlFamily::Postgres => format!("\"{}\".", &schema),
                        _ => "".to_string(),
                    };
                    t.inject_custom(format!(
                        "FOREIGN KEY(city_name, city) REFERENCES {}\"City\"(name, id)",
                        relation_prefix
                    ));
                }
            });
        })
        .await;
    let schema = api.describe().await.expect("describe failed");
    let user_table = schema.get_table("User").expect("couldn't get User table");
    let expected_columns = vec![
        Column {
            name: "city".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "city_name".to_string(),
            tpe: ColumnType {
                raw: varchar_type(api.sql_family(), 255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
    ];

    let expected_indexes = if sql_family.is_mysql() {
        vec![Index {
            name: "city_name".to_owned(),
            columns: vec!["city_name".to_owned(), "city".to_owned()],
            tpe: IndexType::Normal,
        }]
    } else {
        vec![]
    };

    let on_delete_action = match api.sql_family() {
        SqlFamily::Mysql => ForeignKeyAction::Restrict,
        _ => ForeignKeyAction::NoAction,
    };

    assert_eq!(
        user_table,
        &Table {
            name: "User".to_string(),
            columns: expected_columns,
            indices: expected_indexes,
            primary_key: None,
            foreign_keys: vec![ForeignKey {
                constraint_name: match (api.sql_family(), api.connector_name()) {
                    (SqlFamily::Postgres, "postgres12") => Some("User_city_name_city_fkey".to_owned()),
                    (SqlFamily::Postgres, _) => Some("User_city_name_fkey".to_owned()),
                    (SqlFamily::Mysql, _) => Some("User_ibfk_1".to_owned()),
                    (SqlFamily::Sqlite, _) => None,
                },
                columns: vec!["city_name".to_string(), "city".to_string()],
                referenced_columns: vec!["name".to_string(), "id".to_string(),],
                referenced_table: "City".to_string(),
                on_delete_action,
            },],
        }
    );
}

#[test_each_connector]
async fn names_with_hyphens_must_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User-table", |t| {
                t.add_column("column-1", types::integer().nullable(false));
            });
        })
        .await;
    let result = api.describe().await.expect("describing");
    let user_table = result.get_table("User-table").expect("getting User table");
    let expected_columns = vec![Column {
        name: "column-1".to_string(),
        tpe: ColumnType {
            raw: int_type(api.sql_family()),
            family: ColumnTypeFamily::Int,
            arity: ColumnArity::Required,
        },
        default: None,
        auto_increment: false,
    }];
    assert_eq!(user_table.columns, expected_columns);
}

#[test_each_connector]
async fn composite_primary_keys_must_work(api: &TestApi) {
    let sql = match api.sql_family() {
        SqlFamily::Mysql => format!(
            "CREATE TABLE `{0}`.`User` (
                    id INTEGER NOT NULL,
                    name VARCHAR(255) NOT NULL,
                    PRIMARY KEY(id, name)
                )",
            api.db_name()
        ),
        _ => format!(
            "CREATE TABLE \"{0}\".\"User\" (
                    id INTEGER NOT NULL,
                    name VARCHAR(255) NOT NULL,
                    PRIMARY KEY(id, name)
                )",
            api.schema_name()
        ),
    };

    api.database().query_raw(&sql, &[]).await.unwrap();

    let schema = api.describe().await.expect("describe failed");
    let table = schema.get_table("User").expect("couldn't get User table");
    let (exp_int, exp_varchar) = match api.sql_family() {
        SqlFamily::Sqlite => ("INTEGER", "VARCHAR(255)"),
        SqlFamily::Mysql => ("int", "varchar"),
        SqlFamily::Postgres => ("int4", "varchar"),
    };
    let mut expected_columns = vec![
        Column {
            name: "id".to_string(),
            tpe: ColumnType {
                raw: exp_int.to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "name".to_string(),
            tpe: ColumnType {
                raw: exp_varchar.to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },
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
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["id".to_string(), "name".to_string()],
                sequence: None,
            }),
            foreign_keys: vec![],
        }
    );
}

#[test_each_connector]
async fn indices_must_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::primary());
                t.add_column("count", types::integer());
                t.add_index("count", types::index(vec!["count"]));
            });
        })
        .await;
    let result = api.describe().await.expect("describing");
    let user_table = result.get_table("User").expect("getting User table");
    let default = match api.sql_family() {
        SqlFamily::Postgres => Some(DefaultValue::SEQUENCE(format!("nextval('\"User_id_seq\"'::regclass)"))),
        _ => None,
    };
    let expected_columns = vec![
        Column {
            name: "count".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "id".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default,
            auto_increment: true,
        },
    ];
    let pk_sequence = match api.sql_family() {
        SqlFamily::Postgres => Some(Sequence {
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
}

#[test_each_connector]
async fn column_uniqueness_must_be_detected(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("uniq1", types::integer().unique(true));
                t.add_column("uniq2", types::integer());
                t.add_index("uniq", types::index(vec!["uniq2"]).unique(true));
            });
        })
        .await;

    let result = api.describe().await.expect("describing");
    let user_table = result.get_table("User").expect("getting User table");
    let expected_columns = vec![
        Column {
            name: "uniq1".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "uniq2".to_string(),
            tpe: ColumnType {
                raw: int_type(api.sql_family()),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
    ];
    let mut expected_indices = vec![Index {
        name: "uniq".to_string(),
        columns: vec!["uniq2".to_string()],
        tpe: IndexType::Unique,
    }];
    match api.sql_family() {
        SqlFamily::Mysql => expected_indices.push(Index {
            name: "uniq1".to_string(),
            columns: vec!["uniq1".to_string()],
            tpe: IndexType::Unique,
        }),
        SqlFamily::Postgres => expected_indices.insert(
            0,
            Index {
                name: "User_uniq1_key".to_string(),
                columns: vec!["uniq1".to_string()],
                tpe: IndexType::Unique,
            },
        ),
        SqlFamily::Sqlite => expected_indices.push(Index {
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
        user_table.is_column_unique(&user_table.columns[0].name),
        "Column 1 should return true for is_unique"
    );
    assert!(
        user_table.is_column_unique(&user_table.columns[1].name),
        "Column 2 should return true for is_unique"
    );
}

#[test_each_connector]
async fn defaults_must_work(api: &TestApi) {
    api.barrel()
        .execute(|migration| {
            migration.create_table("User", move |t| {
                t.add_column("id", types::integer().default(1).nullable(true));
            });
        })
        .await;

    let result = api.describe().await.expect("describing");
    let user_table = result.get_table("User").expect("getting User table");
    let default = DefaultValue::VALUE(PrismaValue::Int(1));
    let expected_columns = vec![Column {
        name: "id".to_string(),
        tpe: ColumnType {
            raw: int_type(api.sql_family()),
            family: ColumnTypeFamily::Int,
            arity: ColumnArity::Nullable,
        },

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
}

use crate::{common::*, test_api::*};
use barrel::types;
use native_types::{MsSqlType, MsSqlTypeParameter, MySqlType, NativeType, PostgresType};
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use quaint::prelude::{Queryable, SqlFamily};
use serde_json::Value;
use sql_schema_describer::*;
use test_macros::test_each_connector;
use test_setup::connectors::Tags;

mod common;
mod mssql;
mod mysql;
mod postgres;
mod sqlite;
mod test_api;

fn int_full_data_type(api: &TestApi) -> &'static str {
    match api.sql_family() {
        SqlFamily::Postgres => "int4",
        SqlFamily::Sqlite => "INTEGER",
        SqlFamily::Mysql if api.connector_tags().contains(Tags::Mysql8) => "int",
        SqlFamily::Mysql => "int(11)",
        SqlFamily::Mssql => "int",
    }
}

fn int_native_type(api: &TestApi) -> Option<Value> {
    match api.sql_family() {
        SqlFamily::Postgres => Some(PostgresType::Integer.to_json()),
        SqlFamily::Sqlite => None,
        SqlFamily::Mysql if api.connector_tags().contains(Tags::Mysql8) => Some(MySqlType::Int.to_json()),
        SqlFamily::Mysql => Some(MySqlType::Int.to_json()),
        SqlFamily::Mssql => Some(MsSqlType::Int.to_json()),
    }
}

fn varchar_full_data_type(api: &TestApi, length: u64) -> String {
    match api.sql_family() {
        SqlFamily::Postgres => "varchar".to_string(),
        SqlFamily::Sqlite => format!("VARCHAR({})", length),
        SqlFamily::Mysql if api.connector_tags().contains(Tags::Mysql8) => format!("varchar({})", length),
        SqlFamily::Mysql => format!("varchar({})", length),
        SqlFamily::Mssql => format!("varchar({})", length),
    }
}

fn varchar_native_type(api: &TestApi, length: u32) -> Option<Value> {
    match api.sql_family() {
        SqlFamily::Postgres => Some(PostgresType::VarChar(Some(length)).to_json()),
        SqlFamily::Sqlite => None,
        SqlFamily::Mysql if api.connector_tags().contains(Tags::Mysql8) => Some(MySqlType::VarChar(length).to_json()),
        SqlFamily::Mysql => Some(MySqlType::VarChar(length).to_json()),
        SqlFamily::Mssql => Some(MsSqlType::VarChar(Some(MsSqlTypeParameter::Number(length as u16))).to_json()),
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
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "column2".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Nullable,
                native_type: int_native_type(api),
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
            full_data_type: int_full_data_type(api).into(),
            family: ColumnTypeFamily::Int,
            arity: ColumnArity::Required,
            native_type: int_native_type(api),
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
            columns: vec![user_table.column_index_for_bang("city")],
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
                    SqlFamily::Mssql => Some("User_city_fkey".to_owned()),
                },
                columns: vec!["city".to_string()],
                referenced_columns: vec!["id".to_string()],
                referenced_table: "City".to_string(),
                on_delete_action,
                on_update_action: ForeignKeyAction::NoAction,
            }],
        }
    );
}

#[test_each_connector(log = "quaint=info")]
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
                } else if sql_family == SqlFamily::Mssql {
                    t.inject_custom(format!(
                        "FOREIGN KEY(city_name, city) REFERENCES [{}].[City]([name], [id])",
                        schema,
                    ));
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
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "city_name".to_string(),
            tpe: ColumnType {
                full_data_type: varchar_full_data_type(api, 255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: varchar_native_type(api, 255),
            },
            default: None,
            auto_increment: false,
        },
    ];

    let expected_indexes = if sql_family.is_mysql() {
        vec![Index {
            name: "city_name".to_owned(),
            columns: vec![
                user_table.column_index_for_bang("city_name"),
                user_table.column_index_for_bang("city"),
            ],
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
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres if api.connector_tags().contains(Tags::Postgres12) =>
                        Some("User_city_name_city_fkey".to_owned()),
                    SqlFamily::Postgres => Some("User_city_name_fkey".to_owned()),
                    SqlFamily::Mysql => Some("User_ibfk_1".to_owned()),
                    SqlFamily::Sqlite => None,
                    SqlFamily::Mssql => Some("User_city_name_fkey".to_owned()),
                },
                columns: vec!["city_name".to_string(), "city".to_string()],
                referenced_columns: vec!["name".to_string(), "id".to_string(),],
                referenced_table: "City".to_string(),
                on_delete_action,
                on_update_action: ForeignKeyAction::NoAction,
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
            full_data_type: int_full_data_type(api).into(),
            family: ColumnTypeFamily::Int,
            arity: ColumnArity::Required,
            native_type: int_native_type(api),
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
        SqlFamily::Mssql => format!(
            "CREATE TABLE [{}].[User] (
                [id] INT NOT NULL,
                [name] VARCHAR(255) NOT NULL,
                CONSTRAINT [PK_User] PRIMARY KEY ([id], [name])
            )",
            api.schema_name(),
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
    let mut expected_columns = vec![
        Column {
            name: "id".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "name".to_string(),
            tpe: ColumnType {
                full_data_type: varchar_full_data_type(api, 255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: varchar_native_type(api, 255),
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
                constraint_name: match api.sql_family() {
                    SqlFamily::Postgres => Some("User_pkey".into()),
                    SqlFamily::Mssql => Some("PK_User".into()),
                    _ => None,
                }
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
        SqlFamily::Postgres => Some(DefaultValue::sequence("User_id_seq".to_string())),
        _ => None,
    };
    let expected_columns = vec![
        Column {
            name: "id".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },

            default,
            auto_increment: true,
        },
        Column {
            name: "count".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },
            default: None,
            auto_increment: false,
        },
    ];
    let pk_sequence = match api.sql_family() {
        SqlFamily::Postgres => Some(Sequence {
            name: "User_id_seq".to_string(),
        }),
        _ => None,
    };

    assert_eq!("User", user_table.name);
    assert_eq!(expected_columns, user_table.columns);

    assert_eq!(
        vec![Index {
            name: "count".to_string(),
            columns: vec![1],
            tpe: IndexType::Normal,
        }],
        user_table.indices
    );

    assert!(user_table.primary_key.is_some());
    assert_eq!(Vec::<ForeignKey>::new(), user_table.foreign_keys);

    let pk = user_table.primary_key.as_ref().unwrap();

    assert_eq!(vec!["id".to_string()], pk.columns);
    assert_eq!(pk_sequence, pk.sequence);

    match api.sql_family() {
        SqlFamily::Postgres => assert_eq!(Some("User_pkey".to_string()), pk.constraint_name),
        SqlFamily::Mssql => assert!(pk
            .constraint_name
            .as_ref()
            .map(|name| name.starts_with("PK__User__"))
            .unwrap_or(false)),
        _ => assert!(pk.constraint_name.is_none()),
    }
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
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "uniq2".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(api).into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: int_native_type(api),
            },

            default: None,
            auto_increment: false,
        },
    ];
    let mut expected_indices = vec![Index {
        name: "uniq".to_string(),
        columns: vec![user_table.column_index_for_bang("uniq2")],
        tpe: IndexType::Unique,
    }];
    match api.sql_family() {
        SqlFamily::Mysql => expected_indices.push(Index {
            name: "uniq1".to_string(),
            columns: vec![user_table.column_index_for_bang("uniq1")],
            tpe: IndexType::Unique,
        }),
        SqlFamily::Postgres => expected_indices.insert(
            0,
            Index {
                name: "User_uniq1_key".to_string(),
                columns: vec![user_table.column_index_for_bang("uniq1")],
                tpe: IndexType::Unique,
            },
        ),
        SqlFamily::Sqlite => expected_indices.push(Index {
            name: "sqlite_autoindex_User_1".to_string(),
            columns: vec![user_table.column_index_for_bang("uniq1")],
            tpe: IndexType::Unique,
        }),
        SqlFamily::Mssql => expected_indices.insert(
            0,
            Index {
                name: "UQ__User__CD572100A176666B".to_string(),
                columns: vec![user_table.column_index_for_bang("uniq1")],
                tpe: IndexType::Unique,
            },
        ),
    };

    match api.sql_family() {
        SqlFamily::Mssql => {
            assert_eq!(&user_table.name, "User");
            assert_eq!(user_table.columns, expected_columns);

            assert_eq!(user_table.indices.last().unwrap(), expected_indices.last().unwrap());

            let index = user_table.indices.first().unwrap();
            let expected_index = expected_indices.first().unwrap();

            assert!(index.name.starts_with("UQ__User__"));
            assert_eq!(index.columns, expected_index.columns);
            assert_eq!(index.tpe, expected_index.tpe);

            assert!(user_table.primary_key.is_none());
            assert!(user_table.foreign_keys.is_empty());
        }
        _ => {
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
        }
    }

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

    assert_eq!("User", &user_table.name);
    assert_eq!(Vec::<Index>::new(), user_table.indices);
    assert_eq!(Vec::<ForeignKey>::new(), user_table.foreign_keys);
    assert_eq!(None, user_table.primary_key);

    let id = user_table.columns.first().unwrap();

    assert_eq!("id", &id.name);
    assert_eq!(false, id.auto_increment);

    let expected_type = ColumnType {
        full_data_type: int_full_data_type(api).into(),
        family: ColumnTypeFamily::Int,
        arity: ColumnArity::Nullable,
        native_type: int_native_type(api),
    };

    assert_eq!(expected_type, id.tpe);

    let default = id.default.as_ref().unwrap();

    if api.sql_family().is_mssql() {
        assert!(default.constraint_name().unwrap().starts_with("DF__User__id__"));
    }

    assert_eq!(&DefaultKind::VALUE(PrismaValue::Int(1)), default.kind());
}

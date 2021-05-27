mod describers;
mod test_api;

use crate::test_api::*;
use barrel::types;
use native_types::{MsSqlType, MsSqlTypeParameter, MySqlType, NativeType, PostgresType};
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use quaint::prelude::SqlFamily;
use serde_json::Value;
use sql_schema_describer::*;

fn int_full_data_type(api: &TestApi) -> &'static str {
    match api.sql_family() {
        SqlFamily::Postgres if api.is_cockroach() => "int8",
        SqlFamily::Postgres => "int4",
        SqlFamily::Sqlite => "INTEGER",
        SqlFamily::Mysql if api.connector_tags().contains(Tags::Mysql8) => "int",
        SqlFamily::Mysql => "int(11)",
        SqlFamily::Mssql => "int",
    }
}

fn int_native_type(api: &TestApi) -> Option<Value> {
    match api.sql_family() {
        SqlFamily::Postgres if api.is_cockroach() => Some(PostgresType::BigInt.to_json()),
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

#[test_connector]
fn is_required_must_work(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User", |t| {
            t.add_column("column1", types::integer().nullable(false));
            t.add_column("column2", types::integer().nullable(true));
        });
    });

    api.describe().assert_table("User", |t| {
        t.assert_column("column1", |c| c.assert_not_null())
            .assert_column("column2", |c| c.assert_nullable())
    });
}

#[test_connector]
fn foreign_keys_must_work(api: TestApi) {
    let sql_family = api.sql_family();

    api.execute_barrel(|migration| {
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
    });

    let schema = api.describe();

    schema.assert_table("User", |t| {
        let t = t
            .assert_column("city", |c| c.assert_type_is_int_or_bigint())
            .assert_foreign_key_on_columns(&["city"], |fk| fk.assert_references("City", &["id"]));

        if sql_family.is_mysql() {
            t.assert_index_on_columns(&["city"], |idx| idx.assert_name("city"))
        } else {
            t
        }
    });
}

#[test_connector]
fn multi_column_foreign_keys_must_work(api: TestApi) {
    let sql_family = api.sql_family();
    let schema = api.schema_name().to_owned();

    api.execute_barrel(|migration| {
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
    });

    let schema = api.describe();

    schema.assert_table("User", |t| {
        let t = t
            .assert_column("city", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_name", |c| c.assert_type_is_string())
            .assert_foreign_key_on_columns(&["city_name", "city"], |fk| {
                fk.assert_references("City", &["name", "id"])
            });

        if sql_family.is_mysql() {
            t.assert_index_on_columns(&["city_name", "city"], |idx| idx.assert_name("city_name"))
        } else {
            t
        }
    });
}

#[test_connector]
fn names_with_hyphens_must_work(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User-table", |t| {
            t.add_column("column-1", types::integer().nullable(false));
        });
    });

    api.describe().assert_table("User-table", |table| {
        table.assert_column("column-1", |c| c.assert_not_null())
    });
}

#[test_connector]
fn composite_primary_keys_must_work(api: TestApi) {
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

    api.raw_cmd(&sql);

    let schema = api.describe();
    let table = schema.get_table("User").expect("couldn't get User table");
    let mut expected_columns = vec![
        Column {
            name: "id".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(&api).into(),
                family: if api.is_cockroach() {
                    ColumnTypeFamily::BigInt
                } else {
                    ColumnTypeFamily::Int
                },
                arity: ColumnArity::Required,
                native_type: int_native_type(&api),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "name".to_string(),
            tpe: ColumnType {
                full_data_type: varchar_full_data_type(&api, 255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: varchar_native_type(&api, 255),
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
                    SqlFamily::Postgres if api.is_cockroach() => Some("primary".into()),
                    SqlFamily::Postgres => Some("User_pkey".into()),
                    SqlFamily::Mssql => Some("PK_User".into()),
                    _ => None,
                }
            }),
            foreign_keys: vec![],
        }
    );
}

#[test_connector]
fn indices_must_work(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User", move |t| {
            t.add_column("id", types::primary());
            t.add_column("count", types::integer());
            t.add_index("count", types::index(vec!["count"]));
        });
    });
    let result = api.describe();
    let user_table = result.get_table("User").expect("getting User table");
    let default = match api.sql_family() {
        SqlFamily::Postgres if api.is_cockroach() => Some(DefaultValue::db_generated("unique_rowid()")),
        SqlFamily::Postgres => Some(DefaultValue::sequence("User_id_seq".to_string())),
        _ => None,
    };
    let expected_columns = vec![
        Column {
            name: "id".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(&api).into(),
                family: if api.is_cockroach() {
                    ColumnTypeFamily::BigInt
                } else {
                    ColumnTypeFamily::Int
                },
                arity: ColumnArity::Required,
                native_type: int_native_type(&api),
            },

            default,
            auto_increment: true,
        },
        Column {
            name: "count".to_string(),
            tpe: ColumnType {
                full_data_type: int_full_data_type(&api).into(),
                family: if api.is_cockroach() {
                    ColumnTypeFamily::BigInt
                } else {
                    ColumnTypeFamily::Int
                },
                arity: ColumnArity::Required,
                native_type: int_native_type(&api),
            },
            default: None,
            auto_increment: false,
        },
    ];
    let pk_sequence = match api.sql_family() {
        SqlFamily::Postgres if api.is_cockroach() => None,
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
            columns: vec!["count".to_string()],
            tpe: IndexType::Normal,
        }],
        user_table.indices
    );

    assert!(user_table.primary_key.is_some());
    assert_eq!(Vec::<ForeignKey>::new(), user_table.foreign_keys);

    let pk = user_table.primary_key.as_ref().unwrap();

    assert_eq!(pk.columns, &["id"]);
    assert_eq!(pk_sequence, pk.sequence);

    match api.sql_family() {
        SqlFamily::Postgres if api.is_cockroach() => assert_eq!(Some("primary"), pk.constraint_name.as_deref()),
        SqlFamily::Postgres => assert_eq!(Some("User_pkey"), pk.constraint_name.as_deref()),
        SqlFamily::Mssql => assert!(pk
            .constraint_name
            .as_ref()
            .map(|name| name.starts_with("PK__User__"))
            .unwrap_or(false)),
        _ => assert!(pk.constraint_name.is_none()),
    }
}

#[test_connector]
fn column_uniqueness_must_be_detected(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User", move |t| {
            t.add_column("uniq1", types::integer().unique(true));
            t.add_column("uniq2", types::integer());
            t.add_index("uniq", types::index(vec!["uniq2"]).unique(true));
        });
    });

    let schema = api.describe();

    schema.assert_table("User", |t| {
        t.assert_column("uniq1", |c| {
            c.assert_type_is_int_or_bigint()
                .assert_not_null()
                .assert_auto_increment(false)
                .assert_no_default()
        })
        .assert_column("uniq2", |c| {
            c.assert_type_is_int_or_bigint()
                .assert_not_null()
                .assert_no_default()
                .assert_auto_increment(false)
        })
        .assert_foreign_keys_count(0)
        .assert_indexes_count(2)
        .assert_index_on_columns(&["uniq2"], |idx| {
            let idx = idx.assert_is_unique();

            if !api.is_mssql() {
                idx.assert_name("uniq")
            } else {
                idx
            }
        })
        .assert_index_on_columns(&["uniq1"], |idx| idx.assert_is_unique())
    });

    let user_table = schema.table_bang("User");

    assert!(
        user_table.is_column_unique(&user_table.columns[0].name),
        "Column 1 should return true for is_unique"
    );
    assert!(
        user_table.is_column_unique(&user_table.columns[1].name),
        "Column 2 should return true for is_unique"
    );
}

#[test_connector]
fn defaults_must_work(api: TestApi) {
    api.execute_barrel(|migration| {
        migration.create_table("User", move |t| {
            t.add_column("id", types::integer().default(1).nullable(true));
        });
    });

    let result = api.describe();
    let user_table = result.get_table("User").expect("getting User table");

    assert_eq!("User", &user_table.name);
    assert_eq!(Vec::<Index>::new(), user_table.indices);
    assert_eq!(Vec::<ForeignKey>::new(), user_table.foreign_keys);

    if !api.is_cockroach() {
        assert_eq!(None, user_table.primary_key);
    }

    let id = user_table.columns.first().unwrap();

    assert_eq!("id", &id.name);
    assert_eq!(false, id.auto_increment);

    let expected_type = ColumnType {
        full_data_type: int_full_data_type(&api).into(),
        family: if api.is_cockroach() {
            ColumnTypeFamily::BigInt
        } else {
            ColumnTypeFamily::Int
        },
        arity: ColumnArity::Nullable,
        native_type: int_native_type(&api),
    };

    assert_eq!(expected_type, id.tpe);

    let default = id.default.as_ref().unwrap();

    if api.sql_family().is_mssql() {
        assert!(default.constraint_name().unwrap().starts_with("DF__User__id__"));
    }

    if api.is_cockroach() {
        assert_eq!(&DefaultKind::Value(PrismaValue::BigInt(1)), default.kind());
    } else {
        assert_eq!(&DefaultKind::Value(PrismaValue::Int(1)), default.kind());
    }
}

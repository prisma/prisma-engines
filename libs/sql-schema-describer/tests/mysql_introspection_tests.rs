use barrel::{types, Migration};
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

mod common;
mod mysql;

use crate::common::*;
use crate::mysql::*;

#[tokio::test]
async fn all_mysql_column_types_must_work() {
    setup();

    let mut migration = Migration::new().schema(SCHEMA);
    migration.create_table("User", move |t| {
        t.add_column("primary_col", types::primary());
        t.add_column("int_col", types::custom("int"));
        t.add_column("smallint_col", types::custom("smallint"));
        t.add_column("tinyint_col", types::custom("tinyint"));
        t.add_column("mediumint_col", types::custom("mediumint"));
        t.add_column("bigint_col", types::custom("bigint"));
        t.add_column("decimal_col", types::custom("decimal"));
        t.add_column("numeric_col", types::custom("numeric"));
        t.add_column("float_col", types::custom("float"));
        t.add_column("double_col", types::custom("double"));
        t.add_column("date_col", types::custom("date"));
        t.add_column("time_col", types::custom("time"));
        t.add_column("datetime_col", types::custom("datetime"));
        t.add_column("timestamp_col", types::custom("timestamp"));
        t.add_column("year_col", types::custom("year"));
        t.add_column("char_col", types::custom("char"));
        t.add_column("varchar_col", types::custom("varchar(255)"));
        t.add_column("text_col", types::custom("text"));
        t.add_column("tinytext_col", types::custom("tinytext"));
        t.add_column("mediumtext_col", types::custom("mediumtext"));
        t.add_column("longtext_col", types::custom("longtext"));
        t.add_column("enum_col", types::custom("enum('a', 'b')"));
        t.add_column("set_col", types::custom("set('a', 'b')"));
        t.add_column("binary_col", types::custom("binary"));
        t.add_column("varbinary_col", types::custom("varbinary(255)"));
        t.add_column("blob_col", types::custom("blob"));
        t.add_column("tinyblob_col", types::custom("tinyblob"));
        t.add_column("mediumblob_col", types::custom("mediumblob"));
        t.add_column("longblob_col", types::custom("longblob"));
        t.add_column("geometry_col", types::custom("geometry"));
        t.add_column("point_col", types::custom("point"));
        t.add_column("linestring_col", types::custom("linestring"));
        t.add_column("polygon_col", types::custom("polygon"));
        t.add_column("multipoint_col", types::custom("multipoint"));
        t.add_column("multilinestring_col", types::custom("multilinestring"));
        t.add_column("multipolygon_col", types::custom("multipolygon"));
        t.add_column("geometrycollection_col", types::custom("geometrycollection"));
        t.add_column("json_col", types::custom("json"));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    let inspector = get_mysql_describer(&full_sql).await;
    let result = inspector.describe(&SCHEMA.to_string()).await.expect("describing");
    let mut table = result.get_table("User").expect("couldn't get User table").to_owned();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by_key(|c| c.name.to_owned());
    let mut expected_columns = vec![
        Column {
            name: "primary_col".to_string(),
            tpe: ColumnType {
                raw: "int".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: true,
        },
        Column {
            name: "int_col".to_string(),
            tpe: ColumnType {
                raw: "int".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".to_string(),
            tpe: ColumnType {
                raw: "smallint".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint_col".to_string(),
            tpe: ColumnType {
                raw: "tinyint".to_string(),
                family: ColumnTypeFamily::Boolean,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumint_col".to_string(),
            tpe: ColumnType {
                raw: "mediumint".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigint_col".to_string(),
            tpe: ColumnType {
                raw: "bigint".to_string(),
                family: ColumnTypeFamily::Int,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "decimal_col".to_string(),
            tpe: ColumnType {
                raw: "decimal".to_string(),
                family: ColumnTypeFamily::Float,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".to_string(),
            tpe: ColumnType {
                raw: "decimal".to_string(),
                family: ColumnTypeFamily::Float,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".to_string(),
            tpe: ColumnType {
                raw: "float".to_string(),
                family: ColumnTypeFamily::Float,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".to_string(),
            tpe: ColumnType {
                raw: "double".to_string(),
                family: ColumnTypeFamily::Float,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_col".to_string(),
            tpe: ColumnType {
                raw: "date".to_string(),
                family: ColumnTypeFamily::DateTime,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_col".to_string(),
            tpe: ColumnType {
                raw: "time".to_string(),
                family: ColumnTypeFamily::DateTime,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime_col".to_string(),
            tpe: ColumnType {
                raw: "datetime".to_string(),
                family: ColumnTypeFamily::DateTime,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamp_col".to_string(),
            tpe: ColumnType {
                raw: "timestamp".to_string(),
                family: ColumnTypeFamily::DateTime,
            },
            arity: ColumnArity::Required,
            default: Some("CURRENT_TIMESTAMP".to_string()),
            auto_increment: false,
        },
        Column {
            name: "year_col".to_string(),
            tpe: ColumnType {
                raw: "year".to_string(),
                family: ColumnTypeFamily::DateTime,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".to_string(),
            tpe: ColumnType {
                raw: "char".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_col".to_string(),
            tpe: ColumnType {
                raw: "varchar".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "text_col".to_string(),
            tpe: ColumnType {
                raw: "text".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinytext_col".to_string(),
            tpe: ColumnType {
                raw: "tinytext".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumtext_col".to_string(),
            tpe: ColumnType {
                raw: "mediumtext".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "longtext_col".to_string(),
            tpe: ColumnType {
                raw: "longtext".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "enum_col".to_string(),
            tpe: ColumnType {
                raw: "enum".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "set_col".to_string(),
            tpe: ColumnType {
                raw: "set".to_string(),
                family: ColumnTypeFamily::String,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".to_string(),
            tpe: ColumnType {
                raw: "binary".to_string(),
                family: ColumnTypeFamily::Binary,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_col".to_string(),
            tpe: ColumnType {
                raw: "varbinary".to_string(),
                family: ColumnTypeFamily::Binary,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "blob_col".to_string(),
            tpe: ColumnType {
                raw: "blob".to_string(),
                family: ColumnTypeFamily::Binary,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyblob_col".to_string(),
            tpe: ColumnType {
                raw: "tinyblob".to_string(),
                family: ColumnTypeFamily::Binary,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "mediumblob_col".to_string(),
            tpe: ColumnType {
                raw: "mediumblob".to_string(),
                family: ColumnTypeFamily::Binary,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "longblob_col".to_string(),
            tpe: ColumnType {
                raw: "longblob".to_string(),
                family: ColumnTypeFamily::Binary,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "geometry_col".to_string(),
            tpe: ColumnType {
                raw: "geometry".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "point_col".to_string(),
            tpe: ColumnType {
                raw: "point".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "linestring_col".to_string(),
            tpe: ColumnType {
                raw: "linestring".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "polygon_col".to_string(),
            tpe: ColumnType {
                raw: "polygon".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multipoint_col".to_string(),
            tpe: ColumnType {
                raw: "multipoint".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multilinestring_col".to_string(),
            tpe: ColumnType {
                raw: "multilinestring".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "multipolygon_col".to_string(),
            tpe: ColumnType {
                raw: "multipolygon".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "geometrycollection_col".to_string(),
            tpe: ColumnType {
                raw: "geometrycollection".to_string(),
                family: ColumnTypeFamily::Geometric,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
        Column {
            name: "json_col".to_string(),
            tpe: ColumnType {
                raw: "json".to_string(),
                family: ColumnTypeFamily::Json,
            },
            arity: ColumnArity::Required,
            default: None,
            auto_increment: false,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!(
        table,
        Table {
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

#[tokio::test]
async fn mysql_foreign_key_on_delete_must_be_handled() {
    setup();

    // NB: We don't test the SET DEFAULT variety since it isn't supported on InnoDB and will
    // just cause an error
    let sql = format!(
        "CREATE TABLE `{0}`.City (id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY);
         CREATE TABLE `{0}`.User (
            id INTEGER NOT NULL AUTO_INCREMENT PRIMARY KEY,
            city INTEGER, FOREIGN KEY(city) REFERENCES City (id) ON DELETE NO ACTION,
            city_cascade INTEGER, FOREIGN KEY(city_cascade) REFERENCES City (id) ON DELETE CASCADE,
            city_restrict INTEGER, FOREIGN KEY(city_restrict) REFERENCES City (id) ON DELETE RESTRICT,
            city_set_null INTEGER, FOREIGN KEY(city_set_null) REFERENCES City (id) ON DELETE SET NULL
        )",
        SCHEMA
    );
    let inspector = get_mysql_describer(&sql).await;

    let schema = inspector.describe(SCHEMA).await.expect("describing");
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
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_cascade".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_restrict".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_set_null".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                    },
                    arity: ColumnArity::Nullable,
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "id".to_string(),
                    tpe: ColumnType {
                        raw: "int".to_string(),
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
                    constraint_name: Some("User_ibfk_1".to_owned()),
                    columns: vec!["city".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::NoAction,
                },
                ForeignKey {
                    constraint_name: Some("User_ibfk_2".to_owned()),
                    columns: vec!["city_cascade".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::Cascade,
                },
                ForeignKey {
                    constraint_name: Some("User_ibfk_3".to_owned()),
                    columns: vec!["city_restrict".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::Restrict,
                },
                ForeignKey {
                    constraint_name: Some("User_ibfk_4".to_owned()),
                    columns: vec!["city_set_null".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_delete_action: ForeignKeyAction::SetNull,
                },
            ],
        }
    );
}

#[tokio::test]
async fn mysql_multi_field_indexes_must_be_inferred() {
    setup();

    let mut migration = Migration::new().schema(SCHEMA);
    migration.create_table("Employee", move |t| {
        t.add_column("id", types::primary());
        t.add_column("age", types::integer());
        t.add_column("name", types::varchar(200));
        t.add_index("age_and_name_index", types::index(vec!["name", "age"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    let inspector = get_mysql_describer(&full_sql).await;
    let result = inspector.describe(&SCHEMA.to_string()).await.expect("describing");
    let table = result.get_table("Employee").expect("couldn't get Employee table");

    assert_eq!(
        table.indices,
        &[Index {
            name: "age_and_name_index".into(),
            columns: vec!["name".to_owned(), "age".to_owned()],
            tpe: IndexType::Unique
        }]
    );
}

#[tokio::test]
async fn mysql_join_table_unique_indexes_must_be_inferred() {
    setup();

    let mut migration = Migration::new().schema(SCHEMA);

    migration.create_table("Cat", move |t| {
        t.add_column("id", types::primary());
        t.add_column("name", types::text());
    });

    migration.create_table("Human", move |t| {
        t.add_column("id", types::primary());
        t.add_column("name", types::text());
    });

    migration.create_table("CatToHuman", move |t| {
        t.add_column("cat", types::foreign("Cat", "id").nullable(true));
        t.add_column("human", types::foreign("Human", "id").nullable(true));
        t.add_column("relationship", types::text());
        t.add_index("cat_and_human_index", types::index(vec!["cat", "human"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    let inspector = get_mysql_describer(&full_sql).await;
    let result = inspector.describe(&SCHEMA.to_string()).await.expect("describing");
    let table = result.get_table("CatToHuman").expect("couldn't get CatToHuman table");

    assert_eq!(
        table.indices,
        &[Index {
            name: "cat_and_human_index".into(),
            columns: vec!["cat".to_owned(), "human".to_owned()],
            tpe: IndexType::Unique
        }]
    );
}

// When multiple databases exist on a mysql instance, and they share names for foreign key
// constraints, introspecting one database should not yield constraints from the other.
#[tokio::test]
async fn constraints_from_other_databases_should_not_be_introspected() {
    setup();

    let mut other_migration = Migration::new().schema("other_schema");

    other_migration.create_table("User", |t| {
        t.add_column("id", types::primary());
    });
    other_migration.create_table("Post", |t| {
        t.add_column("id", types::primary());
        t.inject_custom("user_id INTEGER, FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE CASCADE");
    });

    let full_sql = other_migration.make::<barrel::backend::MySql>();
    let inspector = get_mysql_describer_for_schema(&full_sql, "other_schema").await;

    let schema = inspector
        .describe(&"other_schema".to_string())
        .await
        .expect("describing");
    let table = schema.table_bang("Post");

    let fks = &table.foreign_keys;

    assert_eq!(
        fks,
        &[ForeignKey {
            constraint_name: Some("Post_ibfk_1".into()),
            columns: vec!["user_id".into()],
            referenced_table: "User".into(),
            referenced_columns: vec!["id".into()],
            on_delete_action: ForeignKeyAction::Cascade,
        }]
    );

    // Now the migration in the current database.

    let mut migration = Migration::new().schema(SCHEMA);

    migration.create_table("User", |t| {
        t.add_column("id", types::primary());
    });

    migration.create_table("Post", |t| {
        t.add_column("id", types::primary());
        t.inject_custom("user_id INTEGER, FOREIGN KEY (`user_id`) REFERENCES `User`(`id`) ON DELETE RESTRICT");
    });

    let full_sql = migration.make::<barrel::backend::MySql>();
    let inspector = get_mysql_describer_for_schema(&full_sql, SCHEMA).await;
    let schema = inspector.describe(&SCHEMA.to_string()).await.expect("describing");
    let table = schema.table_bang("Post");

    let fks = &table.foreign_keys;

    assert_eq!(
        fks,
        &[ForeignKey {
            constraint_name: Some("Post_ibfk_1".into()),
            columns: vec!["user_id".into()],
            referenced_table: "User".into(),
            referenced_columns: vec!["id".into()],
            on_delete_action: ForeignKeyAction::Restrict,
        }]
    );
}

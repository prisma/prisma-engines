mod mssql;
mod test_api;

#[cfg(not(target_os = "macos"))]
use crate::mssql::*;
#[cfg(not(target_os = "macos"))]
use barrel::{types, Migration};
#[cfg(not(target_os = "macos"))]
use pretty_assertions::assert_eq;
#[cfg(not(target_os = "macos"))]
use sql_schema_describer::*;

#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn all_mssql_column_types_must_work() {
    let db_name = "all_mssql_column_types_must_work";

    let mut migration = Migration::new().schema(db_name);
    migration.create_table("User", move |t| {
        t.add_column("primary_col", types::primary());
        t.add_column("bit_col", types::custom("bit"));
        t.add_column("decimal_col", types::custom("decimal"));
        t.add_column("int_col", types::custom("int"));
        t.add_column("money_col", types::custom("money"));
        t.add_column("numeric_col", types::custom("numeric"));
        t.add_column("smallint_col", types::custom("smallint"));
        t.add_column("smallmoney_col", types::custom("smallmoney"));
        t.add_column("tinyint_col", types::custom("tinyint"));
        t.add_column("float_col", types::custom("float(24)"));
        t.add_column("double_col", types::custom("float(53)"));
        t.add_column("date_col", types::custom("date"));
        t.add_column("datetime2_col", types::custom("datetime2"));
        t.add_column("datetime_col", types::custom("datetime"));
        t.add_column("datetimeoffset_col", types::custom("datetimeoffset"));
        t.add_column("smalldatetime_col", types::custom("smalldatetime"));
        t.add_column("time_col", types::custom("time"));
        t.add_column("char_col", types::custom("char(255)"));
        t.add_column("varchar_col", types::custom("varchar(255)"));
        t.add_column("varchar_max_col", types::custom("varchar(max)"));
        t.add_column("text_col", types::custom("text"));
        t.add_column("nvarchar_col", types::custom("nvarchar(255)"));
        t.add_column("nvarchar_max_col", types::custom("nvarchar(max)"));
        t.add_column("ntext_col", types::custom("ntext"));
        t.add_column("binary_col", types::custom("binary(20)"));
        t.add_column("varbinary_col", types::custom("varbinary(20)"));
        t.add_column("varbinary_max_col", types::custom("varbinary(max)"));
        t.add_column("image_col", types::custom("image"));
    });

    let full_sql = migration.make::<barrel::backend::MsSql>();
    let inspector = get_mssql_describer_for_schema(&full_sql, db_name).await;
    let result = inspector.describe(db_name).await.expect("describing");
    let mut table = result.get_table("User").expect("couldn't get User table").to_owned();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by_key(|c| c.name.to_owned());
    let mut expected_columns = vec![
        Column {
            name: "primary_col".to_string(),
            tpe: ColumnType {
                data_type: "int".to_string(),
                full_data_type: "int".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: true,
        },
        Column {
            name: "bit_col".to_string(),
            tpe: ColumnType {
                data_type: "bit".to_string(),
                full_data_type: "bit".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "decimal_col".to_string(),
            tpe: ColumnType {
                data_type: "decimal".to_string(),
                full_data_type: "decimal".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "int_col".to_string(),
            tpe: ColumnType {
                data_type: "int".to_string(),
                full_data_type: "int".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "money_col".to_string(),
            tpe: ColumnType {
                data_type: "money".to_string(),
                full_data_type: "money".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".to_string(),
            tpe: ColumnType {
                data_type: "numeric".to_string(),
                full_data_type: "numeric".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".to_string(),
            tpe: ColumnType {
                data_type: "smallint".to_string(),
                full_data_type: "smallint".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallmoney_col".to_string(),
            tpe: ColumnType {
                data_type: "smallmoney".to_string(),
                full_data_type: "smallmoney".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint_col".to_string(),
            tpe: ColumnType {
                data_type: "tinyint".to_string(),
                full_data_type: "tinyint".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".to_string(),
            tpe: ColumnType {
                data_type: "real".to_string(),
                full_data_type: "real".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".to_string(),
            tpe: ColumnType {
                data_type: "float".to_string(),
                full_data_type: "float".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_col".to_string(),
            tpe: ColumnType {
                data_type: "date".to_string(),
                full_data_type: "date".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime_col".to_string(),
            tpe: ColumnType {
                data_type: "datetime".to_string(),
                full_data_type: "datetime".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime2_col".to_string(),
            tpe: ColumnType {
                data_type: "datetime2".to_string(),
                full_data_type: "datetime2".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetimeoffset_col".to_string(),
            tpe: ColumnType {
                data_type: "datetimeoffset".to_string(),
                full_data_type: "datetimeoffset".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smalldatetime_col".to_string(),
            tpe: ColumnType {
                data_type: "smalldatetime".to_string(),
                full_data_type: "smalldatetime".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_col".to_string(),
            tpe: ColumnType {
                data_type: "time".to_string(),
                full_data_type: "time".to_string(),
                character_maximum_length: None,
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".to_string(),
            tpe: ColumnType {
                data_type: "char".to_string(),
                full_data_type: "char".to_string(),
                character_maximum_length: Some(255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_col".to_string(),
            tpe: ColumnType {
                data_type: "varchar".to_string(),
                full_data_type: "varchar".to_string(),
                character_maximum_length: Some(255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_max_col".to_string(),
            tpe: ColumnType {
                data_type: "varchar".to_string(),
                full_data_type: "varchar".to_string(),
                character_maximum_length: Some(-1),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "text_col".to_string(),
            tpe: ColumnType {
                data_type: "text".to_string(),
                full_data_type: "text".to_string(),
                character_maximum_length: Some(2147483647),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "nvarchar_col".to_string(),
            tpe: ColumnType {
                data_type: "nvarchar".to_string(),
                full_data_type: "nvarchar".to_string(),
                character_maximum_length: Some(255),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "nvarchar_max_col".to_string(),
            tpe: ColumnType {
                data_type: "nvarchar".to_string(),
                full_data_type: "nvarchar".to_string(),
                character_maximum_length: Some(-1),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "ntext_col".to_string(),
            tpe: ColumnType {
                data_type: "ntext".to_string(),
                full_data_type: "ntext".to_string(),
                character_maximum_length: Some(1073741823),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".to_string(),
            tpe: ColumnType {
                data_type: "binary".to_string(),
                full_data_type: "binary".to_string(),
                character_maximum_length: Some(20),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_col".to_string(),
            tpe: ColumnType {
                data_type: "varbinary".to_string(),
                full_data_type: "varbinary".to_string(),
                character_maximum_length: Some(20),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_max_col".to_string(),
            tpe: ColumnType {
                data_type: "varbinary".to_string(),
                full_data_type: "varbinary".to_string(),
                character_maximum_length: Some(-1),

                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "image_col".to_string(),
            tpe: ColumnType {
                data_type: "image".to_string(),
                full_data_type: "image".to_string(),
                character_maximum_length: Some(2147483647),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
            },

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
                constraint_name: None,
            }),
            foreign_keys: vec![],
        }
    );
}

#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn mssql_foreign_key_on_delete_must_be_handled() {
    let db_name = "mssql_foreign_key_on_delete_must_be_handled";

    let sql = format!(
        "CREATE TABLE [{0}].[City] (id INT NOT NULL IDENTITY(1,1) PRIMARY KEY);
         CREATE TABLE [{0}].[User] (
            id INT NOT NULL IDENTITY(1,1) PRIMARY KEY,
            city INT, FOREIGN KEY(city) REFERENCES [{0}].[City] (id) ON DELETE NO ACTION,
            city_cascade INT, FOREIGN KEY(city_cascade) REFERENCES [{0}].[City] (id) ON DELETE CASCADE
        )",
        db_name
    );
    let inspector = get_mssql_describer_for_schema(&sql, db_name).await;

    let schema = inspector.describe(db_name).await.expect("describing");
    let mut table = schema.get_table("User").expect("get User table").to_owned();
    table.foreign_keys.sort_unstable_by_key(|fk| fk.columns.clone());

    assert_eq!(
        table,
        Table {
            name: "User".to_string(),
            columns: vec![
                Column {
                    name: "id".to_string(),
                    tpe: ColumnType {
                        data_type: "int".to_string(),
                        full_data_type: "int".to_string(),
                        character_maximum_length: None,
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                    },

                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "city".to_string(),
                    tpe: ColumnType {
                        data_type: "int".to_string(),
                        full_data_type: "int".to_string(),
                        character_maximum_length: None,
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_cascade".to_string(),
                    tpe: ColumnType {
                        data_type: "int".to_string(),
                        full_data_type: "int".to_string(),
                        character_maximum_length: None,
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
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
            foreign_keys: vec![
                ForeignKey {
                    constraint_name: Some("User_ibfk_1".to_owned()),
                    columns: vec!["city".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_update_action: ForeignKeyAction::NoAction,
                    on_delete_action: ForeignKeyAction::NoAction,
                },
                ForeignKey {
                    constraint_name: Some("User_ibfk_2".to_owned()),
                    columns: vec!["city_cascade".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_update_action: ForeignKeyAction::Cascade,
                    on_delete_action: ForeignKeyAction::Cascade,
                },
            ],
        }
    );
}

#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn mssql_multi_field_indexes_must_be_inferred() {
    let db_name = "mssql_multi_field_indexes_must_be_inferred";

    let mut migration = Migration::new().schema(db_name);
    migration.create_table("Employee", move |t| {
        t.add_column("id", types::primary());
        t.add_column("age", types::integer());
        t.add_column("name", types::varchar(200));
        t.add_index("age_and_name_index", types::index(vec!["name", "age"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MsSql>();
    let inspector = get_mssql_describer_for_schema(&full_sql, db_name).await;
    let result = inspector.describe(db_name).await.expect("describing");
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

#[cfg(not(target_os = "macos"))]
#[tokio::test]
async fn mssql_join_table_unique_indexes_must_be_inferred() {
    let db_name = "mssql_join_table_unique_indexes_must_be_inferred";

    let mut migration = Migration::new().schema(db_name);

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

    let full_sql = migration.make::<barrel::backend::MsSql>();
    let inspector = get_mssql_describer_for_schema(&full_sql, db_name).await;
    let result = inspector.describe(db_name).await.expect("describing");
    let table = result.get_table("CatToHuman").expect("couldn't get CatToHuman table");

    assert_eq!(
        table.indices,
        &[Index {
            name: "cat_and_human_index".into(),
            columns: vec!["cat".to_owned(), "human".to_owned()],
            tpe: IndexType::Unique,
        }]
    );
}

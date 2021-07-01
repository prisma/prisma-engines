use crate::test_api::*;
use barrel::{types, Migration};
use native_types::{MsSqlType, MsSqlTypeParameter::*, NativeType};
use pretty_assertions::assert_eq;
use sql_schema_describer::{mssql::SqlSchemaDescriber, *};

#[test_connector(tags(Mssql))]
fn udts_can_be_described(api: TestApi) {
    let conn = api.database();
    let db_name = api.db_name();

    let types = &[
        "bigint",
        "binary(255)",
        "bit",
        "char(255)",
        "date",
        "datetime",
        "datetime2",
        "datetimeoffset",
        "decimal(10,2)",
        "real",
        "float",
        "image",
        "int",
        "money",
        "nchar(100)",
        "ntext",
        "numeric(10,5)",
        "nvarchar(100)",
        "nvarchar(max)",
        "real",
        "smalldatetime",
        "smallint",
        "smallmoney",
        "text",
        "time",
        "tinyint",
        "uniqueidentifier",
        "varbinary(50)",
        "varbinary(max)",
        "varchar(100)",
        "varchar(max)",
    ];

    for r#type in types {
        api.block_on(test_setup::reset_mssql_schema(conn, db_name)).unwrap();

        api.raw_cmd(&format!("CREATE TYPE {}.a FROM {}", db_name, r#type));

        let result = api.describe();
        let udt = result
            .get_user_defined_type("a")
            .expect("couldn't get a type")
            .to_owned();

        assert_eq!("a", &udt.name);
        assert_eq!(Some(*r#type), udt.definition.as_deref());
    }
}

#[test_connector(tags(Mssql))]
fn views_can_be_described(api: TestApi) {
    let db_name = api.db_name();
    let conn = api.database();

    api.block_on(test_setup::reset_mssql_schema(conn, db_name)).unwrap();

    api.raw_cmd(&format!("CREATE TABLE {}.a (a_id int)", db_name));
    api.raw_cmd(&format!("CREATE TABLE {}.b (b_id int)", db_name));

    let create_view = format!(
        r#"
            CREATE VIEW {0}.ab AS
            SELECT a_id
            FROM {0}.a
            UNION ALL
            SELECT b_id
            FROM {0}.b"#,
        db_name
    );

    api.raw_cmd(&create_view);

    let inspector = SqlSchemaDescriber::new(conn);
    let result = api.block_on(inspector.describe(db_name)).unwrap();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    assert_eq!("ab", &view.name);
    assert_eq!(create_view, view.definition.unwrap());
}

#[test_connector(tags(Mssql))]
fn procedures_can_be_described(api: TestApi) {
    let sql = format!(
        "CREATE PROCEDURE [{}].foo @ID INT AS SELECT DB_NAME(@ID) AS bar",
        api.db_name()
    );

    api.raw_cmd(&sql);

    let result = api.describe();
    let procedure = result.get_procedure("foo").unwrap();

    assert_eq!("foo", &procedure.name);
    assert_eq!(Some(sql), procedure.definition);
}

#[test_connector(tags(Mssql))]
fn all_mssql_column_types_must_work(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
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
        t.add_column("xml_col", types::custom("xml"));
    });

    let full_sql = migration.make::<barrel::backend::MsSql>();
    api.raw_cmd(&full_sql);
    let result = api.describe();
    let mut table = result.get_table("User").expect("couldn't get User table").to_owned();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by_key(|c| c.name.to_owned());
    let mut expected_columns = vec![
        Column {
            name: "primary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "int".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Int.to_json()),
            },

            default: None,
            auto_increment: true,
        },
        Column {
            name: "bit_col".to_string(),
            tpe: ColumnType {
                full_data_type: "bit".to_string(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Bit.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "decimal_col".to_string(),
            tpe: ColumnType {
                full_data_type: "decimal(18,0)".to_string(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Decimal(Some((18, 0))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "int_col".to_string(),
            tpe: ColumnType {
                full_data_type: "int".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Int.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "money_col".to_string(),
            tpe: ColumnType {
                full_data_type: "money".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Money.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".to_string(),
            tpe: ColumnType {
                full_data_type: "numeric(18,0)".to_string(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Decimal(Some((18, 0))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "smallint".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::SmallInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallmoney_col".to_string(),
            tpe: ColumnType {
                full_data_type: "smallmoney".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::SmallMoney.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "tinyint_col".to_string(),
            tpe: ColumnType {
                full_data_type: "tinyint".to_string(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::TinyInt.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".to_string(),
            tpe: ColumnType {
                full_data_type: "real".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Real.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".to_string(),
            tpe: ColumnType {
                full_data_type: "float(53)".to_string(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Float(Some(53)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_col".to_string(),
            tpe: ColumnType {
                full_data_type: "date".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Date.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime_col".to_string(),
            tpe: ColumnType {
                full_data_type: "datetime".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::DateTime.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetime2_col".to_string(),
            tpe: ColumnType {
                full_data_type: "datetime2".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::DateTime2.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "datetimeoffset_col".to_string(),
            tpe: ColumnType {
                full_data_type: "datetimeoffset".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::DateTimeOffset.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "smalldatetime_col".to_string(),
            tpe: ColumnType {
                full_data_type: "smalldatetime".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::SmallDateTime.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_col".to_string(),
            tpe: ColumnType {
                full_data_type: "time".to_string(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Time.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".to_string(),
            tpe: ColumnType {
                full_data_type: "char(255)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Char(Some(255)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varchar(255)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::VarChar(Some(Number(255))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varchar_max_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varchar(max)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::VarChar(Some(Max)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "text_col".to_string(),
            tpe: ColumnType {
                full_data_type: "text".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Text.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "nvarchar_col".to_string(),
            tpe: ColumnType {
                full_data_type: "nvarchar(255)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::NVarChar(Some(Number(255))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "nvarchar_max_col".to_string(),
            tpe: ColumnType {
                full_data_type: "nvarchar(max)".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::NVarChar(Some(Max)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "ntext_col".to_string(),
            tpe: ColumnType {
                full_data_type: "ntext".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::NText.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "binary(20)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Binary(Some(20)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varbinary(20)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::VarBinary(Some(Number(20))).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "varbinary_max_col".to_string(),
            tpe: ColumnType {
                full_data_type: "varbinary(max)".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::VarBinary(Some(Max)).to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "image_col".to_string(),
            tpe: ColumnType {
                full_data_type: "image".to_string(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Image.to_json()),
            },

            default: None,
            auto_increment: false,
        },
        Column {
            name: "xml_col".to_string(),
            tpe: ColumnType {
                full_data_type: "xml".to_string(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(MsSqlType::Xml.to_json()),
            },

            default: None,
            auto_increment: false,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!("User", &table.name);
    assert_eq!(expected_columns, table.columns);
    assert_eq!(Vec::<Index>::new(), table.indices);
    assert_eq!(Vec::<ForeignKey>::new(), table.foreign_keys);

    let pk = table.primary_key.as_ref().unwrap();

    assert_eq!(vec!["primary_col".to_string()], pk.columns);
    assert_eq!(None, pk.sequence);
    assert!(pk
        .constraint_name
        .as_ref()
        .map(|s| s.starts_with("PK__User__"))
        .unwrap_or(false));
}

#[test_connector(tags(Mssql))]
fn mssql_cross_schema_references_are_not_allowed(api: TestApi) {
    let db_name = api.db_name();
    let secondary = "mssql_foreign_key_on_delete_must_be_handled_B";
    let conn = api.database();

    api.block_on(test_setup::reset_mssql_schema(conn, secondary)).unwrap();

    let sql = format!(
        "
            CREATE TABLE [{1}].[City] (id INT NOT NULL IDENTITY(1,1), CONSTRAINT [PK__City] PRIMARY KEY ([id]));
            CREATE TABLE [{0}].[User]
            (
                id           INT NOT NULL IDENTITY (1,1),
                city         INT,
                city_cascade INT,
                CONSTRAINT [FK__city] FOREIGN KEY (city) REFERENCES [{1}].[City] (id) ON DELETE NO ACTION,
                CONSTRAINT [PK__User] PRIMARY KEY ([id])
            );
        ",
        db_name, secondary
    );

    api.raw_cmd(&sql);
    let err = api.describe_error();

    assert_eq!(
        "Illegal cross schema reference from `mssql_cross_schema_references_are_not_allowed.User` to `mssql_foreign_key_on_delete_must_be_handled_B.City` in constraint `FK__city`. Foreign keys between database schemas are not supported in Prisma. Please follow the GitHub ticket: https://github.com/prisma/prisma/issues/1175".to_string(),
        format!("{}", err),
    );
}

#[test_connector(tags(Mssql))]
fn mssql_foreign_key_on_delete_must_be_handled(api: TestApi) {
    let sql = format!(
        "
            CREATE TABLE [{0}].[City] (id INT NOT NULL IDENTITY(1,1), CONSTRAINT [PK__City] PRIMARY KEY ([id]));
            CREATE TABLE [{0}].[User]
            (
                id           INT NOT NULL IDENTITY (1,1),
                city         INT,
                city_cascade INT,
                CONSTRAINT [FK__city] FOREIGN KEY (city) REFERENCES [{0}].[City] (id) ON DELETE NO ACTION,
                CONSTRAINT [FK__city_cascade] FOREIGN KEY (city_cascade) REFERENCES [{0}].[City] (id) ON DELETE CASCADE,
                CONSTRAINT [PK__User] PRIMARY KEY ([id])
            );
        ",
        api.db_name()
    );

    api.raw_cmd(&sql);

    let schema = api.describe();
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
                        full_data_type: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Required,
                        native_type: Some(MsSqlType::Int.to_json()),
                    },

                    default: None,
                    auto_increment: true,
                },
                Column {
                    name: "city".to_string(),
                    tpe: ColumnType {
                        full_data_type: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                        native_type: Some(MsSqlType::Int.to_json()),
                    },
                    default: None,
                    auto_increment: false,
                },
                Column {
                    name: "city_cascade".to_string(),
                    tpe: ColumnType {
                        full_data_type: "int".to_string(),
                        family: ColumnTypeFamily::Int,
                        arity: ColumnArity::Nullable,
                        native_type: Some(MsSqlType::Int.to_json()),
                    },
                    default: None,
                    auto_increment: false,
                },
            ],
            indices: vec![],
            primary_key: Some(PrimaryKey {
                columns: vec!["id".to_string()],
                sequence: None,
                constraint_name: Some("PK__User".into()),
            }),
            foreign_keys: vec![
                ForeignKey {
                    constraint_name: Some("FK__city".to_owned()),
                    columns: vec!["city".to_string()],
                    referenced_columns: vec!["id".to_string()],
                    referenced_table: "City".to_string(),
                    on_update_action: ForeignKeyAction::NoAction,
                    on_delete_action: ForeignKeyAction::NoAction,
                },
                ForeignKey {
                    constraint_name: Some("FK__city_cascade".to_owned()),
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

#[test_connector(tags(Mssql))]
fn mssql_multi_field_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());
    migration.create_table("Employee", move |t| {
        t.add_column("id", types::primary());
        t.add_column("age", types::integer());
        t.add_column("name", types::varchar(200));
        t.add_index("age_and_name_index", types::index(vec!["name", "age"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MsSql>();
    api.raw_cmd(&full_sql);
    let result = api.describe();
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

#[test_connector(tags(Mssql))]
fn mssql_join_table_unique_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new().schema(api.db_name());

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
    api.raw_cmd(&full_sql);
    let result = api.describe();
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

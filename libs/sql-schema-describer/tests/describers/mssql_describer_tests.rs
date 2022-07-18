use crate::test_api::*;
use barrel::{types, Migration};
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

#[test_connector(tags(Mssql))]
fn udts_can_be_described(api: TestApi) {
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
        api.raw_cmd(&format!("DROP TYPE IF EXISTS a; CREATE TYPE a FROM {type}"));

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
    let view_definition = r#"
        CREATE VIEW ab AS
            SELECT a_id
            FROM a
            UNION ALL
            SELECT b_id
            FROM b;
    "#;
    let create_tables = r#"
        CREATE TABLE a (a_id int);
        CREATE TABLE b (b_id int);
    "#;
    api.raw_cmd(create_tables);
    api.raw_cmd(view_definition);

    let result = api.describe();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    assert_eq!("ab", &view.name);
    assert_eq!(view_definition, view.definition.unwrap());
}

#[test_connector(tags(Mssql))]
fn procedures_can_be_described(api: TestApi) {
    let sql = "CREATE PROCEDURE [dbo].foo @ID INT AS SELECT DB_NAME(@ID) AS bar";
    api.raw_cmd(sql);

    let result = api.describe();
    let procedure = result.get_procedure("foo").unwrap();

    assert_eq!("foo", &procedure.name);
    assert_eq!(Some(sql), procedure.definition.as_deref());
}

#[test_connector(tags(Mssql))]
fn all_mssql_column_types_must_work(api: TestApi) {
    let mut migration = Migration::new();
    migration.create_table("User", move |t| {
        t.add_column("primary_col", types::integer());
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
        t.inject_custom("CONSTRAINT \"thepk\" PRIMARY KEY (primary_col)");
    });

    let full_sql = migration.make::<barrel::backend::MsSql>();
    api.raw_cmd(&full_sql);
    let expectation = expect![[r#"
        SqlSchema {
            tables: [
                Table {
                    name: "User",
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bit_col",
                        tpe: ColumnType {
                            full_data_type: "bit",
                            family: Boolean,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Bit",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "decimal_col",
                        tpe: ColumnType {
                            full_data_type: "decimal(18,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            18,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Int",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "money_col",
                        tpe: ColumnType {
                            full_data_type: "money",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Money",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "numeric_col",
                        tpe: ColumnType {
                            full_data_type: "numeric(18,0)",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Array([
                                        Number(
                                            18,
                                        ),
                                        Number(
                                            0,
                                        ),
                                    ]),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "smallint_col",
                        tpe: ColumnType {
                            full_data_type: "smallint",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "smallmoney_col",
                        tpe: ColumnType {
                            full_data_type: "smallmoney",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallMoney",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "tinyint_col",
                        tpe: ColumnType {
                            full_data_type: "tinyint",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "TinyInt",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "float_col",
                        tpe: ColumnType {
                            full_data_type: "real",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Real",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "double_col",
                        tpe: ColumnType {
                            full_data_type: "float(53)",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Float": Number(
                                        53,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "date_col",
                        tpe: ColumnType {
                            full_data_type: "date",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Date",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "datetime2_col",
                        tpe: ColumnType {
                            full_data_type: "datetime2",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "DateTime2",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "datetime_col",
                        tpe: ColumnType {
                            full_data_type: "datetime",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "DateTime",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "datetimeoffset_col",
                        tpe: ColumnType {
                            full_data_type: "datetimeoffset",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "DateTimeOffset",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "smalldatetime_col",
                        tpe: ColumnType {
                            full_data_type: "smalldatetime",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallDateTime",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "time_col",
                        tpe: ColumnType {
                            full_data_type: "time",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Time",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "char_col",
                        tpe: ColumnType {
                            full_data_type: "char(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Char": Number(
                                        255,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varchar_col",
                        tpe: ColumnType {
                            full_data_type: "varchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Object({
                                        "Number": Number(
                                            255,
                                        ),
                                    }),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varchar_max_col",
                        tpe: ColumnType {
                            full_data_type: "varchar(max)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": String(
                                        "Max",
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "text_col",
                        tpe: ColumnType {
                            full_data_type: "text",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Text",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "nvarchar_col",
                        tpe: ColumnType {
                            full_data_type: "nvarchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "NVarChar": Object({
                                        "Number": Number(
                                            255,
                                        ),
                                    }),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "nvarchar_max_col",
                        tpe: ColumnType {
                            full_data_type: "nvarchar(max)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "NVarChar": String(
                                        "Max",
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "ntext_col",
                        tpe: ColumnType {
                            full_data_type: "ntext",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "NText",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "binary_col",
                        tpe: ColumnType {
                            full_data_type: "binary(20)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Binary": Number(
                                        20,
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varbinary_col",
                        tpe: ColumnType {
                            full_data_type: "varbinary(20)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarBinary": Object({
                                        "Number": Number(
                                            20,
                                        ),
                                    }),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "varbinary_max_col",
                        tpe: ColumnType {
                            full_data_type: "varbinary(max)",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarBinary": String(
                                        "Max",
                                    ),
                                }),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "image_col",
                        tpe: ColumnType {
                            full_data_type: "image",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Image",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "xml_col",
                        tpe: ColumnType {
                            full_data_type: "xml",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Xml",
                                ),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "thepk",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: ColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Mssql))]
fn mssql_cross_schema_references_are_not_allowed(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA mssql_foreign_key_on_delete_must_be_handled_B");

    let sql = format!(
        "
            CREATE TABLE [{0}].[City] (id INT NOT NULL IDENTITY(1,1), CONSTRAINT [PK__City] PRIMARY KEY ([id]));
            CREATE TABLE [dbo].[User]
            (
                id           INT NOT NULL IDENTITY (1,1),
                city         INT,
                city_cascade INT,
                CONSTRAINT [FK__city] FOREIGN KEY (city) REFERENCES [{0}].[City] (id) ON DELETE NO ACTION,
                CONSTRAINT [PK__User] PRIMARY KEY ([id])
            );
        ",
        "mssql_foreign_key_on_delete_must_be_handled_B"
    );

    api.raw_cmd(&sql);
    let err = api.describe_error();

    assert_eq!(
        "Illegal cross schema reference from `dbo.User` to `mssql_foreign_key_on_delete_must_be_handled_B.City` in constraint `FK__city`. Foreign keys between database schemas are not supported in Prisma. Please follow the GitHub ticket: https://github.com/prisma/prisma/issues/1175",
        err.to_string(),
    );
}

#[test_connector(tags(Mssql))]
fn primary_key_sort_order_desc_is_handled(api: TestApi) {
    let sql = formatdoc! {r#"
        CREATE TABLE [{}].[A]
        (
            a INT NOT NULL,
            b INT NOT NULL,
            CONSTRAINT [PK__a_b] PRIMARY KEY (a ASC, b DESC)
        );
    "#, api.schema_name()};

    api.raw_cmd(&sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();

    assert_eq!(2, table.primary_key_columns_count());

    let columns = table.primary_key_columns().unwrap().collect::<Vec<_>>();

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Asc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Desc), columns[1].sort_order());
}

#[test_connector(tags(Mssql))]
fn index_sort_order_desc_is_handled(api: TestApi) {
    let sql = formatdoc! {r#"
        CREATE TABLE [{schema}].[A]
        (
            id INT PRIMARY KEY,
            a INT NOT NULL,
            b INT NOT NULL
        );

        CREATE INDEX [A_idx] ON [{schema}].[A] (a DESC, b ASC);
    "#, schema = api.schema_name()};

    api.raw_cmd(&sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let index = table.indexes().next().unwrap();

    assert_eq!(2, index.columns().len());

    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Desc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Asc), columns[1].sort_order());
}

#[test_connector(tags(Mssql))]
fn mssql_foreign_key_on_delete_must_be_handled(api: TestApi) {
    let sql = "
        CREATE TABLE [dbo].[City] (id INT NOT NULL IDENTITY(1,1), CONSTRAINT [PK__City] PRIMARY KEY ([id]));
    CREATE TABLE [dbo].[User]
        (
            id           INT NOT NULL IDENTITY (1,1),
            city         INT,
            city_cascade INT,
            CONSTRAINT [FK__city] FOREIGN KEY (city) REFERENCES [dbo].[City] (id) ON DELETE NO ACTION,
            CONSTRAINT [FK__city_cascade] FOREIGN KEY (city_cascade) REFERENCES [dbo].[City] (id) ON DELETE CASCADE,
            CONSTRAINT [PK__User] PRIMARY KEY ([id])
        );
    ";

    api.raw_cmd(sql);
    let schema = api.describe();
    let table = schema.table_walker("User").unwrap();
    let expectations = [
        ("city", ForeignKeyAction::NoAction),
        ("city_cascade", ForeignKeyAction::Cascade),
    ];
    for (colname, action) in expectations {
        let column = table.column(colname).unwrap().id;
        let fk = table.foreign_key_for_column(column).unwrap();
        assert_eq!(action, fk.on_delete_action());
    }
}

#[test_connector(tags(Mssql))]
fn mssql_multi_field_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new();
    migration.create_table("Employee", move |t| {
        t.add_column("id", types::primary());
        t.add_column("age", types::integer());
        t.add_column("name", types::varchar(200));
        t.add_index("age_and_name_index", types::index(vec!["name", "age"]).unique(true));
    });

    let full_sql = migration.make::<barrel::backend::MsSql>();
    api.raw_cmd(&full_sql);
    let result = api.describe();
    result.assert_table("Employee", |t| {
        t.assert_index_on_columns(&["name", "age"], |idx| idx.assert_name("age_and_name_index"))
    });
}

#[test_connector(tags(Mssql))]
fn mssql_join_table_unique_indexes_must_be_inferred(api: TestApi) {
    let mut migration = Migration::new();

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
    result.assert_table("CatToHuman", |t| {
        t.assert_index_on_columns(&["cat", "human"], |idx| idx.assert_name("cat_and_human_index"))
    });
}

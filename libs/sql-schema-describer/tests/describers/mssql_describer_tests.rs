use crate::test_api::*;
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
    let sql = r#"
        CREATE TABLE [User] (
            [primary_col] INTEGER,
            [bit_col] bit,
            [decimal_col] decimal,
            [int_col] int,
            [money_col] money,
            [numeric_col] numeric,
            [smallint_col] smallint,
            [smallmoney_col] smallmoney,
            [tinyint_col] tinyint,
            [float_col] float(24),
            [double_col] float(53),
            [date_col] date,
            [datetime2_col] datetime2,
            [datetime_col] datetime,
            [datetimeoffset_col] datetimeoffset,
            [smalldatetime_col] smalldatetime,
            [time_col] time,
            [char_col] char(255),
            [varchar_col] varchar(255),
            [varchar_max_col] varchar(max),
            [text_col] text,
            [nvarchar_col] nvarchar(255),
            [nvarchar_max_col] nvarchar(max),
            [ntext_col] ntext,
            [binary_col] binary(20),
            [varbinary_col] varbinary(20),
            [varbinary_max_col] varbinary(max),
            [image_col] image,
            [xml_col] xml,
            CONSTRAINT "thepk" PRIMARY KEY (primary_col)
        );
    "#;
    api.raw_cmd(sql);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: [],
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                },
            ],
            enums: [],
            enum_variants: [],
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
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
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
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        default_value_id: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            default_values: [],
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

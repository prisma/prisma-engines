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
            namespaces: {
                "dbo",
            },
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
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
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "thepk",
                    tpe: PrimaryKey,
                    predicate: None,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: TableColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
            runtime_namespace: None,
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

    let expected = expect![
        "The schema of the introspected database was inconsistent: Cross schema references are only allowed when the target schema is listed in the schemas property of your datasource. `dbo.User` points to `mssql_foreign_key_on_delete_must_be_handled_B.City` in constraint `FK__city`. Please add `mssql_foreign_key_on_delete_must_be_handled_B` to your `schemas` property and run this command again."
    ];

    expected.assert_eq(&err.to_string());
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

// multi schema

#[test_connector(tags(Mssql))]
fn multiple_schemas_with_same_table_names_are_described(api: TestApi) {
    api.raw_cmd("CREATE SCHEMA schema_0");
    api.raw_cmd(
        "CREATE TABLE [schema_0].[Table_0] ([id_0] INT, int INT, CONSTRAINT [Table_0_pkey] PRIMARY KEY (id_0))",
    );

    api.raw_cmd("CREATE Schema schema_1");
    api.raw_cmd(
        "CREATE TABLE [schema_1].[Table_0] ([id_1] INT, int INT, CONSTRAINT [Table_0_pkey] PRIMARY KEY (id_1))",
    );

    let schema = api.describe_with_schemas(&["schema_0", "schema_1"]);

    let expected_schema = expect![[r#"
        SqlSchema {
            namespaces: {
                "schema_0",
                "schema_1",
            },
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Table_0",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Table_0",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id_0",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "id_1",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "int",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "Table_0_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "Table_0_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: TableColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        1,
                    ),
                    column_id: TableColumnId(
                        2,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
            runtime_namespace: None,
        }
    "#]];

    expected_schema.assert_debug_eq(&schema);
}

#[test_connector(tags(Mssql))]
fn multiple_schemas_with_same_foreign_key_are_described(api: TestApi) {
    api.raw_cmd("CREATE Schema schema_0");
    api.raw_cmd("CREATE Schema schema_1");

    let schema = indoc! {r#"
        CREATE TABLE schema_0.Table_0 (other INT, id_0 INT, CONSTRAINT Table_0_pkey PRIMARY KEY (id_0));
        CREATE TABLE schema_0.Table_1 (id_1 INT IDENTITY, o_id_0 INT, CONSTRAINT Table_1_pkey PRIMARY KEY (id_1));
        ALTER TABLE schema_0.Table_1 ADD CONSTRAINT fk_0 FOREIGN KEY (o_id_0) REFERENCES schema_0.Table_0 (id_0);

        CREATE TABLE schema_1.Table_0 (id_2 INT IDENTITY, CONSTRAINT Table_0_pkey PRIMARY KEY (id_2));
        CREATE TABLE schema_1.Table_1 (id_3 INT IDENTITY, o_id_0 INT, CONSTRAINT Table_1_pkey PRIMARY KEY (id_3));
        ALTER TABLE schema_1.Table_1 ADD CONSTRAINT fk_0 FOREIGN KEY (o_id_0) REFERENCES schema_1.Table_0 (id_2);

        CREATE TABLE schema_1.Table_2 (id_4 INT IDENTITY, o_id_0 INT, CONSTRAINT Table_2_pkey PRIMARY KEY (id_4));
        ALTER TABLE schema_1.Table_2 ADD CONSTRAINT fk_1 FOREIGN KEY (o_id_0) REFERENCES schema_0.Table_0 (id_0);
    "#};

    api.raw_cmd(schema);

    let schema = api.describe_with_schemas(&["schema_0", "schema_1"]);

    let expected_schema = expect![[r#"
        SqlSchema {
            namespaces: {
                "schema_0",
                "schema_1",
            },
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Table_0",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Table_0",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Table_1",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Table_1",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Table_2",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "other",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id_0",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "id_2",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
                    },
                ),
                (
                    TableId(
                        2,
                    ),
                    Column {
                        name: "id_3",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
                    },
                ),
                (
                    TableId(
                        2,
                    ),
                    Column {
                        name: "o_id_0",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        3,
                    ),
                    Column {
                        name: "id_1",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
                    },
                ),
                (
                    TableId(
                        3,
                    ),
                    Column {
                        name: "o_id_0",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        4,
                    ),
                    Column {
                        name: "id_4",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: true,
                        description: None,
                    },
                ),
                (
                    TableId(
                        4,
                    ),
                    Column {
                        name: "o_id_0",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [
                ForeignKey {
                    constrained_table: TableId(
                        3,
                    ),
                    referenced_table: TableId(
                        0,
                    ),
                    constraint_name: Some(
                        "fk_0",
                    ),
                    on_delete_action: NoAction,
                    on_update_action: NoAction,
                },
                ForeignKey {
                    constrained_table: TableId(
                        4,
                    ),
                    referenced_table: TableId(
                        0,
                    ),
                    constraint_name: Some(
                        "fk_1",
                    ),
                    on_delete_action: NoAction,
                    on_update_action: NoAction,
                },
            ],
            table_default_values: [],
            view_default_values: [],
            foreign_key_columns: [
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        0,
                    ),
                    constrained_column: TableColumnId(
                        6,
                    ),
                    referenced_column: TableColumnId(
                        1,
                    ),
                },
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        0,
                    ),
                    constrained_column: TableColumnId(
                        4,
                    ),
                    referenced_column: TableColumnId(
                        2,
                    ),
                },
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        1,
                    ),
                    constrained_column: TableColumnId(
                        8,
                    ),
                    referenced_column: TableColumnId(
                        1,
                    ),
                },
            ],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "Table_0_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "Table_0_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
                Index {
                    table_id: TableId(
                        3,
                    ),
                    index_name: "Table_1_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
                Index {
                    table_id: TableId(
                        2,
                    ),
                    index_name: "Table_1_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
                Index {
                    table_id: TableId(
                        4,
                    ),
                    index_name: "Table_2_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: TableColumnId(
                        1,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        1,
                    ),
                    column_id: TableColumnId(
                        2,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        2,
                    ),
                    column_id: TableColumnId(
                        5,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        3,
                    ),
                    column_id: TableColumnId(
                        3,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        4,
                    ),
                    column_id: TableColumnId(
                        7,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
            runtime_namespace: None,
        }
    "#]];

    expected_schema.assert_debug_eq(&schema);
}

#[test_connector(tags(Mssql))]
fn partial_indexes_are_described(api: TestApi) {
    let sql = r#"
        CREATE TABLE [User] (
            id INT NOT NULL,
            email NVARCHAR(255) NOT NULL,
            active BIT NOT NULL CONSTRAINT [User_active_df] DEFAULT 1,
            CONSTRAINT [User_pkey] PRIMARY KEY (id)
        );

        CREATE UNIQUE INDEX [User_email_active_idx] ON [User] (email) WHERE active = 1;
    "#;

    api.raw_cmd(sql);
    let expected = expect![[r#"
        SqlSchema {
            namespaces: {
                "dbo",
            },
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
            ],
            enums: [],
            enum_variants: [],
            table_columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "email",
                        tpe: ColumnType {
                            full_data_type: "nvarchar(255)",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "active",
                        tpe: ColumnType {
                            full_data_type: "bit",
                            family: Boolean,
                            arity: Required,
                            native_type: Some(
                                NativeTypeInstance(..),
                            ),
                        },
                        auto_increment: false,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        2,
                    ),
                    DefaultValue {
                        kind: Value(
                            Boolean(
                                true,
                            ),
                        ),
                        constraint_name: Some(
                            "User_active_df",
                        ),
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "User_email_active_idx",
                    tpe: Unique,
                    predicate: Some(
                        "([active]=(1))",
                    ),
                },
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "User_pkey",
                    tpe: PrimaryKey,
                    predicate: None,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: TableColumnId(
                        1,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        1,
                    ),
                    column_id: TableColumnId(
                        0,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            check_constraints: [],
            views: [],
            view_columns: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
            runtime_namespace: None,
        }
    "#]];
    expected.assert_debug_eq(&api.describe());
}

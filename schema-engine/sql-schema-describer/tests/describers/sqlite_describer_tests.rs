use crate::test_api::*;
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

#[test_connector(tags(Sqlite))]
fn multi_column_foreign_keys_must_work(api: TestApi) {
    let sql = r#"
        CREATE TABLE "City" (
            id INTEGER PRIMARY KEY,
            name VARCHAR(255)
        );

        CREATE TABLE "User" (
            city INTEGER,
            city_name VARCHAR(255),

            FOREIGN KEY (city_name, city) REFERENCES "City"(name, id)
        );
    "#;
    api.raw_cmd(sql);
    let schema = api.describe();

    schema.assert_table("User", |t| {
        t.assert_column("city", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_name", |c| c.assert_type_is_string())
            .assert_foreign_key_on_columns(&["city_name", "city"], |fk| {
                fk.assert_references("City", &["name", "id"])
            })
    });
}

#[test_connector(tags(Sqlite))]
fn views_can_be_described(api: TestApi) {
    let full_sql = r#"
        CREATE TABLE a (a_id int);
        CREATE TABLE b (b_id int);
        CREATE VIEW ab AS SELECT a_id FROM a UNION ALL SELECT b_id FROM b;
    "#;

    api.raw_cmd(full_sql);
    let result = api.describe();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    let expected_sql = "CREATE VIEW ab AS SELECT a_id FROM a UNION ALL SELECT b_id FROM b";

    assert_eq!("ab", &view.name);
    assert_eq!(expected_sql, &view.definition.unwrap());
}

#[test_connector(tags(Sqlite))]
fn sqlite_column_types_must_work(api: TestApi) {
    let sql = r#"
        CREATE TABLE "User" (
            int_col int not null,
            int4_col INTEGER NOT NULL,
            text_col TEXT NOT NULL,
            real_col REAL NOT NULL,
            primary_col INTEGER PRIMARY KEY,
            decimal_col DECIMAL (5, 3) NOT NULL
        );
    "#;
    api.raw_cmd(sql);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {},
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
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int",
                            family: Int,
                            arity: Required,
                            native_type: None,
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
                        name: "int4_col",
                        tpe: ColumnType {
                            full_data_type: "integer",
                            family: Int,
                            arity: Required,
                            native_type: None,
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
                            arity: Required,
                            native_type: None,
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
                        name: "real_col",
                        tpe: ColumnType {
                            full_data_type: "real",
                            family: Float,
                            arity: Required,
                            native_type: None,
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
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "integer",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        auto_increment: true,
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
                            full_data_type: "decimal (5, 3)",
                            family: Decimal,
                            arity: Required,
                            native_type: None,
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
                    index_name: "",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: TableColumnId(
                        4,
                    ),
                    sort_order: None,
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

#[test_connector(tags(Sqlite))]
fn sqlite_foreign_key_on_delete_must_be_handled(api: TestApi) {
    use sql_schema_describer::ForeignKeyAction::*;
    let sql = "
        CREATE TABLE City (id INTEGER NOT NULL PRIMARY KEY);
        CREATE TABLE User (
            id INTEGER NOT NULL PRIMARY KEY,
            city INTEGER REFERENCES City(id) ON DELETE NO ACTION,
            city_cascade INTEGER REFERENCES City(id) ON DELETE CASCADE,
            city_restrict INTEGER REFERENCES City (id) ON DELETE RESTRICT,
            city_set_default INTEGER REFERENCES City(id) ON DELETE SET DEFAULT,
            city_set_null INTEGER REFERENCES City(id) ON DELETE SET NULL
        )";

    api.raw_cmd(sql);

    let expectations = [
        ("city", NoAction),
        ("city_cascade", Cascade),
        ("city_restrict", Restrict),
        ("city_set_default", SetDefault),
        ("city_set_null", SetNull),
    ];

    let schema = api.describe();
    let table = schema.table_walker("User").unwrap();

    for (colname, expected_action) in expectations.into_iter() {
        let column = table.column(colname).unwrap().id;
        let action = table.foreign_key_for_column(column).unwrap().on_delete_action();
        assert_eq!(action, expected_action);
    }
}

#[test_connector(tags(Sqlite))]
fn sqlite_text_primary_keys_must_be_inferred_on_table_and_not_as_separate_indexes(api: TestApi) {
    let sql = r#"
        CREATE TABLE "User" (
            int4_col INTEGER,
            text_col TEXT,
            real_col FLOAT,
            primary_col TEXT,

            PRIMARY KEY ("primary_col")
        );
    "#;
    api.raw_cmd(sql);

    let result = api.describe();
    let table = result.table_walker("User").unwrap();
    assert!(result.indexes_count() == 1);
    assert!(table.primary_key_columns_count() == 1);
    assert!(table.primary_key_columns().unwrap().next().unwrap().name() == "primary_col");
}

#[test_connector(tags(Sqlite))]
fn escaped_quotes_in_string_defaults_must_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE "string_defaults_test" (
            regular VARCHAR NOT NULL DEFAULT 'meow, says the cat',
            escaped VARCHAR NOT NULL DEFAULT '"That''s a lot of fish!"
- Godzilla, 1998'
        );
    "#;

    api.raw_cmd(create_table);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {},
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "string_defaults_test",
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
                        name: "regular",
                        tpe: ColumnType {
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: None,
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
                        name: "escaped",
                        tpe: ColumnType {
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: None,
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
                        0,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "meow, says the cat",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        1,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "\"That's a lot of fish!\"\n- Godzilla, 1998",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
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

#[test_connector(tags(Sqlite))]
fn backslashes_in_string_literals(api: TestApi) {
    let create_table = r#"
        CREATE TABLE "test" (
            model_name_space VARCHAR(255) NOT NULL DEFAULT 'xyz\Datasource\Model'
        );
    "#;

    api.raw_cmd(create_table);

    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {},
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "test",
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
                        name: "model_name_space",
                        tpe: ColumnType {
                            full_data_type: "varchar(255)",
                            family: String,
                            arity: Required,
                            native_type: None,
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
                        0,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "xyz\\Datasource\\Model",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
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

#[test_connector(tags(Sqlite))]
fn broken_relations_are_filtered_out(api: TestApi) {
    let setup = r#"
        PRAGMA foreign_keys=OFF;

        CREATE TABLE "platypus" (
            id INTEGER PRIMARY KEY
        );

        CREATE TABLE "dog" (
            id INTEGER PRIMARY KEY,
            bestFriendId INTEGER REFERENCES "cat"("id"),
            realBestFriendId INTEGER REFERENCES "platypus"("id"),
            otherBestFriendId INTEGER REFERENCES "goat"("id")
        );

        PRAGMA foreign_keys=ON;
    "#;

    api.raw_cmd(setup);

    // the relation to platypus should be the only foreign key on dog
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {},
            tables: [
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "dog",
                    properties: BitFlags<TableProperties> {
                        bits: 0b0,
                    },
                    description: None,
                },
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "platypus",
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
                            full_data_type: "integer",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        auto_increment: true,
                        description: None,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bestFriendId",
                        tpe: ColumnType {
                            full_data_type: "integer",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
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
                        name: "realBestFriendId",
                        tpe: ColumnType {
                            full_data_type: "integer",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
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
                        name: "otherBestFriendId",
                        tpe: ColumnType {
                            full_data_type: "integer",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
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
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "integer",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        auto_increment: true,
                        description: None,
                    },
                ),
            ],
            foreign_keys: [
                ForeignKey {
                    constrained_table: TableId(
                        0,
                    ),
                    referenced_table: TableId(
                        1,
                    ),
                    constraint_name: None,
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
                        2,
                    ),
                    referenced_column: TableColumnId(
                        4,
                    ),
                },
            ],
            indexes: [
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "",
                    tpe: PrimaryKey,
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
                    sort_order: None,
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        1,
                    ),
                    column_id: TableColumnId(
                        4,
                    ),
                    sort_order: None,
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

#[test_connector(tags(Sqlite))]
fn index_sort_order_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY,
            a  INT NOT NULL,
            b  INT NOT NULL
        );

        CREATE INDEX foo ON A (a DESC, b ASC);
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let index = table.indexes().nth(1).unwrap();

    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(2, columns.len());

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Desc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Asc), columns[1].sort_order());
}

// See https://www.sqlite.org/lang_createtable.html for the exact logic.
#[test_connector(tags(Sqlite))]
fn integer_primary_keys_autoincrement(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE "A" (
            id INT PRIMARY KEY,
            published BOOLEAN
        );

        CREATE TABLE "B" (
            id integer primary key,
            age INTEGER
        );

        CREATE TABLE "C" (
            pk INTEGER PRIMARY KEY,
            name STRING
        );
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let expected = expect![[r#"
        [
            (
                "A",
                [
                    false,
                ],
            ),
            (
                "B",
                [
                    true,
                ],
            ),
            (
                "C",
                [
                    true,
                ],
            ),
        ]
    "#]];
    let found = schema
        .table_walkers()
        .map(|t| {
            (
                t.name(),
                t.primary_key_columns()
                    .unwrap()
                    .map(|c| c.as_column().is_autoincrement())
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();

    expected.assert_debug_eq(&found);
}

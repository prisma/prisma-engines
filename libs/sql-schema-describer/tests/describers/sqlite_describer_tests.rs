use crate::test_api::*;
use barrel::{types, Migration};
use indoc::indoc;
use pretty_assertions::assert_eq;
use sql_schema_describer::*;

#[test_connector(tags(Sqlite))]
fn multi_column_foreign_keys_must_work(api: TestApi) {
    let sql_family = api.sql_family();

    api.execute_barrel(|migration| {
        migration.create_table("City", move |t| {
            t.add_column("id", types::primary());
            t.add_column("name", types::varchar(255));
        });
        migration.create_table("User", move |t| {
            t.add_column("city", types::integer());
            t.add_column("city_name", types::varchar(255));

            t.inject_custom("FOREIGN KEY(city_name, city) REFERENCES \"City\"(name, id)");
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
    let mut migration = Migration::new();
    migration.create_table("User", move |t| {
        t.inject_custom("int_col int not null");
        t.add_column("int4_col", types::integer());
        t.add_column("text_col", types::text());
        t.add_column("real_col", types::float());
        t.add_column("primary_col", types::primary());
        t.inject_custom("decimal_col decimal (5, 3) not null");
    });

    let full_sql = migration.make::<barrel::backend::Sqlite>();
    api.raw_cmd(&full_sql);
    let expectation = expect![[r#"
        SqlSchema {
            tables: [
                Table {
                    name: "User",
                    indices: [],
                    primary_key: Some(
                        PrimaryKey {
                            columns: [
                                PrimaryKeyColumn {
                                    name: "primary_col",
                                    length: None,
                                    sort_order: None,
                                },
                            ],
                            constraint_name: None,
                        },
                    ),
                },
            ],
            enums: [],
            columns: [
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
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "int4_col",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Required,
                            native_type: None,
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
                            full_data_type: "TEXT",
                            family: String,
                            arity: Required,
                            native_type: None,
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
                        name: "real_col",
                        tpe: ColumnType {
                            full_data_type: "REAL",
                            family: Float,
                            arity: Required,
                            native_type: None,
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
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: true,
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
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Sqlite))]
fn sqlite_foreign_key_on_delete_must_be_handled(api: TestApi) {
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
    let expectation = expect![[r#"
        SqlSchema {
            tables: [
                Table {
                    name: "City",
                    indices: [],
                    primary_key: Some(
                        PrimaryKey {
                            columns: [
                                PrimaryKeyColumn {
                                    name: "id",
                                    length: None,
                                    sort_order: None,
                                },
                            ],
                            constraint_name: None,
                        },
                    ),
                },
                Table {
                    name: "User",
                    indices: [],
                    primary_key: Some(
                        PrimaryKey {
                            columns: [
                                PrimaryKeyColumn {
                                    name: "id",
                                    length: None,
                                    sort_order: None,
                                },
                            ],
                            constraint_name: None,
                        },
                    ),
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "city",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "city_cascade",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "city_restrict",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "city_set_default",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "city_set_null",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [
                (
                    TableId(
                        1,
                    ),
                    ForeignKey {
                        constraint_name: None,
                        columns: [
                            "city",
                        ],
                        referenced_table: TableId(
                            0,
                        ),
                        referenced_columns: [
                            "id",
                        ],
                        on_delete_action: NoAction,
                        on_update_action: NoAction,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    ForeignKey {
                        constraint_name: None,
                        columns: [
                            "city_cascade",
                        ],
                        referenced_table: TableId(
                            0,
                        ),
                        referenced_columns: [
                            "id",
                        ],
                        on_delete_action: Cascade,
                        on_update_action: NoAction,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    ForeignKey {
                        constraint_name: None,
                        columns: [
                            "city_restrict",
                        ],
                        referenced_table: TableId(
                            0,
                        ),
                        referenced_columns: [
                            "id",
                        ],
                        on_delete_action: Restrict,
                        on_update_action: NoAction,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    ForeignKey {
                        constraint_name: None,
                        columns: [
                            "city_set_default",
                        ],
                        referenced_table: TableId(
                            0,
                        ),
                        referenced_columns: [
                            "id",
                        ],
                        on_delete_action: SetDefault,
                        on_update_action: NoAction,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    ForeignKey {
                        constraint_name: None,
                        columns: [
                            "city_set_null",
                        ],
                        referenced_table: TableId(
                            0,
                        ),
                        referenced_columns: [
                            "id",
                        ],
                        on_delete_action: SetNull,
                        on_update_action: NoAction,
                    },
                ),
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
        }
    "#]];
    api.expect_schema(expectation);
}

#[test_connector(tags(Sqlite))]
fn sqlite_text_primary_keys_must_be_inferred_on_table_and_not_as_separate_indexes(api: TestApi) {
    let mut migration = Migration::new();
    migration.create_table("User", move |t| {
        t.add_column("int4_col", types::integer());
        t.add_column("text_col", types::text());
        t.add_column("real_col", types::float());
        t.add_column("primary_col", types::text());

        // Simulate how we create primary keys in the migrations engine.
        t.inject_custom("PRIMARY KEY (\"primary_col\")");
    });
    let full_sql = migration.make::<barrel::backend::Sqlite>();
    api.raw_cmd(&full_sql);

    let result = api.describe();

    let (_, table) = result.table_bang("User");

    assert!(table.indices.is_empty());

    assert_eq!(
        table.primary_key.as_ref().unwrap(),
        &PrimaryKey {
            columns: vec![PrimaryKeyColumn::new("primary_col")],
            constraint_name: None,
        }
    );
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
            tables: [
                Table {
                    name: "string_defaults_test",
                    indices: [],
                    primary_key: None,
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "regular",
                        tpe: ColumnType {
                            full_data_type: "VARCHAR",
                            family: String,
                            arity: Required,
                            native_type: None,
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "meow, says the cat",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "escaped",
                        tpe: ColumnType {
                            full_data_type: "VARCHAR",
                            family: String,
                            arity: Required,
                            native_type: None,
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "\"That's a lot of fish!\"\n- Godzilla, 1998",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
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
            tables: [
                Table {
                    name: "test",
                    indices: [],
                    primary_key: None,
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "model_name_space",
                        tpe: ColumnType {
                            full_data_type: "VARCHAR(255)",
                            family: String,
                            arity: Required,
                            native_type: None,
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "xyz\\Datasource\\Model",
                                    ),
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
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
            tables: [
                Table {
                    name: "dog",
                    indices: [],
                    primary_key: Some(
                        PrimaryKey {
                            columns: [
                                PrimaryKeyColumn {
                                    name: "id",
                                    length: None,
                                    sort_order: None,
                                },
                            ],
                            constraint_name: None,
                        },
                    ),
                },
                Table {
                    name: "platypus",
                    indices: [],
                    primary_key: Some(
                        PrimaryKey {
                            columns: [
                                PrimaryKeyColumn {
                                    name: "id",
                                    length: None,
                                    sort_order: None,
                                },
                            ],
                            constraint_name: None,
                        },
                    ),
                },
            ],
            enums: [],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "bestFriendId",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
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
                        name: "realBestFriendId",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
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
                        name: "otherBestFriendId",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Nullable,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "INTEGER",
                            family: Int,
                            arity: Required,
                            native_type: None,
                        },
                        default: None,
                        auto_increment: true,
                    },
                ),
            ],
            foreign_keys: [
                (
                    TableId(
                        0,
                    ),
                    ForeignKey {
                        constraint_name: None,
                        columns: [
                            "realBestFriendId",
                        ],
                        referenced_table: TableId(
                            1,
                        ),
                        referenced_columns: [
                            "id",
                        ],
                        on_delete_action: NoAction,
                        on_update_action: NoAction,
                    },
                ),
            ],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
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
    let index = table.indexes().next().unwrap();

    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(2, columns.len());

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Desc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Asc), columns[1].sort_order());
}

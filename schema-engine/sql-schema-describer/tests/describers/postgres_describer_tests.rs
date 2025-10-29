mod cockroach_describer_tests;

use crate::test_api::*;
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use sql_schema_describer::{postgres::PostgresSchemaExt, *};

#[test_connector(tags(Postgres))]
fn postgres_skips_nonexisting_namespaces(api: TestApi) {
    let full_sql = r#"
        CREATE SCHEMA "one";
    "#;

    api.raw_cmd(full_sql);
    let schema = api.describe_with_schemas(&["one", "two"]);

    schema.assert_namespace("one").assert_not_namespace("two");
}

#[test_connector(tags(Postgres))]
fn postgres_skips_ignored_namespaces(api: TestApi) {
    let full_sql = r#"
        CREATE SCHEMA "one";
        CREATE SCHEMA "two";
    "#;

    api.raw_cmd(full_sql);
    let schema = api.describe_with_schemas(&["one"]);

    schema.assert_namespace("one").assert_not_namespace("two");
}

#[test_connector(tags(Postgres))]
fn postgres_skips_public_when_ignored_namespaces(api: TestApi) {
    let full_sql = r#"
        CREATE SCHEMA "one";
        CREATE SCHEMA IF NOT EXISTS "public";
    "#;

    api.raw_cmd(full_sql);
    let schema = api.describe_with_schemas(&["one"]);

    schema.assert_namespace("one").assert_not_namespace("public");
}

#[test_connector(tags(Postgres))]
fn postgres_many_namespaces(api: TestApi) {
    let full_sql = r#"
        CREATE SCHEMA "one";
        CREATE SCHEMA "two";
        CREATE SCHEMA "three";
    "#;

    api.raw_cmd(full_sql);
    let schema = api.describe_with_schemas(&["one", "two", "three"]);

    schema
        .assert_namespace("one")
        .assert_namespace("two")
        .assert_namespace("three");
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn views_can_be_described(api: TestApi) {
    let full_sql = r#"
        CREATE TABLE a (a_id int);
        CREATE TABLE b (b_id int);
        CREATE VIEW ab AS SELECT a_id FROM a UNION ALL SELECT b_id FROM b;
    "#;

    api.raw_cmd(full_sql);
    let result = api.describe();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    let expected_sql = " SELECT a.a_id\n   FROM a\nUNION ALL\n SELECT b.b_id AS a_id\n   FROM b;";

    assert_eq!("ab", &view.name);
    assert_eq!(expected_sql, view.definition.unwrap());
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn all_postgres_column_types_must_work(api: TestApi) {
    let sql = r#"
        CREATE TABLE "User" (
            array_bin_col BYTEA[],
            array_bool_col BOOLEAN[],
            array_date_col DATE[],
            array_double_col DOUBLE PRECISION[],
            array_float_col FLOAT[],
            array_int_col INTEGER[],
            array_text_col TEXT[],
            array_varchar_col VARCHAR(255)[],
            bigint_col BIGINT,
            bigserial_col BIGSERIAL,
            bit_col BIT,
            bit_varying_col BIT VARYING(1),
            binary_col BYTEA,
            boolean_col BOOLEAN,
            box_col BOX,
            char_col CHARACTER(1),
            circle_col CIRCLE,
            date_time_col DATE,
            double_col DOUBLE PRECISION,
            float_col FLOAT,
            int_col INTEGER,
            line_col LINE,
            lseg_col LSEG,
            numeric_col NUMERIC,
            path_col PATH,
            pg_lsn_col PG_LSN,
            polygon_col POLYGON,
            smallint_col SMALLINT,
            smallserial_col SMALLSERIAL,
            serial_col SERIAL,
            primary_col SERIAL PRIMARY KEY,
            string1_col TEXT,
            string2_col VARCHAR(1),
            time_col TIME,
            timetz_col TIMETZ,
            timestamp_col TIMESTAMP,
            timestamptz_col TIMESTAMPTZ,
            tsquery_col TSQUERY,
            tsvector_col TSVECTOR,
            txid_col TXID_SNAPSHOT,
            json_col JSON,
            jsonb_col JSONB,
            uuid_col UUID
        );
    "#;
    api.raw_cmd(sql);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {
                "public",
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
                        name: "array_bin_col",
                        tpe: ColumnType {
                            full_data_type: "_bytea",
                            family: Binary,
                            arity: List,
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
                        name: "array_bool_col",
                        tpe: ColumnType {
                            full_data_type: "_bool",
                            family: Boolean,
                            arity: List,
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
                        name: "array_date_col",
                        tpe: ColumnType {
                            full_data_type: "_date",
                            family: DateTime,
                            arity: List,
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
                        name: "array_double_col",
                        tpe: ColumnType {
                            full_data_type: "_float8",
                            family: Float,
                            arity: List,
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
                        name: "array_float_col",
                        tpe: ColumnType {
                            full_data_type: "_float8",
                            family: Float,
                            arity: List,
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
                        name: "array_int_col",
                        tpe: ColumnType {
                            full_data_type: "_int4",
                            family: Int,
                            arity: List,
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
                        name: "array_text_col",
                        tpe: ColumnType {
                            full_data_type: "_text",
                            family: String,
                            arity: List,
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
                        name: "array_varchar_col",
                        tpe: ColumnType {
                            full_data_type: "_varchar",
                            family: String,
                            arity: List,
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
                        name: "bigint_col",
                        tpe: ColumnType {
                            full_data_type: "int8",
                            family: BigInt,
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
                        name: "bigserial_col",
                        tpe: ColumnType {
                            full_data_type: "int8",
                            family: BigInt,
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
                        0,
                    ),
                    Column {
                        name: "bit_col",
                        tpe: ColumnType {
                            full_data_type: "bit",
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
                        name: "bit_varying_col",
                        tpe: ColumnType {
                            full_data_type: "varbit",
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
                            full_data_type: "bytea",
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
                        name: "boolean_col",
                        tpe: ColumnType {
                            full_data_type: "bool",
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
                        name: "box_col",
                        tpe: ColumnType {
                            full_data_type: "box",
                            family: Unsupported(
                                "box",
                            ),
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
                        name: "char_col",
                        tpe: ColumnType {
                            full_data_type: "bpchar",
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
                        name: "circle_col",
                        tpe: ColumnType {
                            full_data_type: "circle",
                            family: Unsupported(
                                "circle",
                            ),
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
                        name: "date_time_col",
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
                        name: "double_col",
                        tpe: ColumnType {
                            full_data_type: "float8",
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
                        name: "float_col",
                        tpe: ColumnType {
                            full_data_type: "float8",
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
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        name: "line_col",
                        tpe: ColumnType {
                            full_data_type: "line",
                            family: Unsupported(
                                "line",
                            ),
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
                        name: "lseg_col",
                        tpe: ColumnType {
                            full_data_type: "lseg",
                            family: Unsupported(
                                "lseg",
                            ),
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
                        name: "numeric_col",
                        tpe: ColumnType {
                            full_data_type: "numeric",
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
                        name: "path_col",
                        tpe: ColumnType {
                            full_data_type: "path",
                            family: Unsupported(
                                "path",
                            ),
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
                        name: "pg_lsn_col",
                        tpe: ColumnType {
                            full_data_type: "pg_lsn",
                            family: Unsupported(
                                "pg_lsn",
                            ),
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
                        name: "polygon_col",
                        tpe: ColumnType {
                            full_data_type: "polygon",
                            family: Unsupported(
                                "polygon",
                            ),
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
                        name: "smallint_col",
                        tpe: ColumnType {
                            full_data_type: "int2",
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
                        name: "smallserial_col",
                        tpe: ColumnType {
                            full_data_type: "int2",
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
                        0,
                    ),
                    Column {
                        name: "serial_col",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        0,
                    ),
                    Column {
                        name: "primary_col",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        0,
                    ),
                    Column {
                        name: "string1_col",
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
                        name: "string2_col",
                        tpe: ColumnType {
                            full_data_type: "varchar",
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
                        name: "timetz_col",
                        tpe: ColumnType {
                            full_data_type: "timetz",
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
                        name: "timestamp_col",
                        tpe: ColumnType {
                            full_data_type: "timestamp",
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
                        name: "timestamptz_col",
                        tpe: ColumnType {
                            full_data_type: "timestamptz",
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
                        name: "tsquery_col",
                        tpe: ColumnType {
                            full_data_type: "tsquery",
                            family: Unsupported(
                                "tsquery",
                            ),
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
                        name: "tsvector_col",
                        tpe: ColumnType {
                            full_data_type: "tsvector",
                            family: Unsupported(
                                "tsvector",
                            ),
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
                        name: "txid_col",
                        tpe: ColumnType {
                            full_data_type: "txid_snapshot",
                            family: Unsupported(
                                "txid_snapshot",
                            ),
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
                        name: "json_col",
                        tpe: ColumnType {
                            full_data_type: "json",
                            family: Json,
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
                        name: "jsonb_col",
                        tpe: ColumnType {
                            full_data_type: "jsonb",
                            family: Json,
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
                        name: "uuid_col",
                        tpe: ColumnType {
                            full_data_type: "uuid",
                            family: Uuid,
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
            table_default_values: [
                (
                    TableColumnId(
                        9,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "User_bigserial_col_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        28,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "User_smallserial_col_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        29,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "User_serial_col_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        30,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "User_primary_col_seq",
                        ),
                        constraint_name: None,
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
                    index_name: "User_pkey",
                    tpe: PrimaryKey,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: TableColumnId(
                        30,
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

    let result = api.describe();
    let ext = extract_ext(&result);
    let expected_ext = expect![[r#"
        PostgresSchemaExt {
            opclasses: [],
            indexes: [
                (
                    IndexId(
                        0,
                    ),
                    BTree,
                ),
            ],
            expression_indexes: [],
            index_null_position: {},
            constraint_options: {
                Index(
                    IndexId(
                        0,
                    ),
                ): BitFlags<ConstraintOption> {
                    bits: 0b0,
                },
            },
            table_options: [
                {},
            ],
            exclude_constraints: [],
            sequences: [
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_bigserial_col_seq",
                    start_value: 1,
                    min_value: 1,
                    max_value: 9223372036854775807,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 0,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_primary_col_seq",
                    start_value: 1,
                    min_value: 1,
                    max_value: 2147483647,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 0,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_serial_col_seq",
                    start_value: 1,
                    min_value: 1,
                    max_value: 2147483647,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 0,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "User_smallserial_col_seq",
                    start_value: 1,
                    min_value: 1,
                    max_value: 32767,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 0,
                    virtual: false,
                },
            ],
            extensions: [
                DatabaseExtension {
                    name: "plpgsql",
                    schema: "pg_catalog",
                    version: "1.0",
                    relocatable: false,
                },
            ],
        }
    "#]];
    expected_ext.assert_debug_eq(&ext);
}

#[test_connector(tags(Postgres))]
fn cross_schema_references_are_not_allowed(api: TestApi) {
    let schema2 = format!("{}_2", api.schema_name());

    let sql = format!(
        "DROP SCHEMA IF EXISTS \"{schema2}\" CASCADE;
         CREATE SCHEMA \"{schema2}\";
         CREATE TABLE \"{schema2}\".\"City\" (id INT PRIMARY KEY);
         CREATE TABLE \"User\" (
            id INT PRIMARY KEY,
            city INT REFERENCES \"{schema2}\".\"City\" (id) ON DELETE NO ACTION
        );
        ",
    );

    api.raw_cmd(&sql);

    let err = api.describe_error();

    let expected = expect![
        "The schema of the introspected database was inconsistent: Cross schema references are only allowed when the target schema is listed in the schemas property of your datasource. `public.User` points to `public_2.City` in constraint `User_city_fkey`. Please add `public_2` to your `schemas` property and run this command again."
    ];

    expected.assert_eq(&err.to_string());
}

#[test_connector(tags(Postgres))]
fn postgres_foreign_key_on_delete_must_be_handled(api: TestApi) {
    let sql = format!(
        "CREATE TABLE \"{0}\".\"City\" (id INT PRIMARY KEY);
         CREATE TABLE \"{0}\".\"User\" (
            id INT PRIMARY KEY,
            city INT REFERENCES \"{0}\".\"City\" (id) ON DELETE NO ACTION,
            city_cascade INT REFERENCES \"{0}\".\"City\" (id) ON DELETE CASCADE,
            city_restrict INT REFERENCES \"{0}\".\"City\" (id) ON DELETE RESTRICT,
            city_set_null INT REFERENCES \"{0}\".\"City\" (id) ON DELETE SET NULL,
            city_set_default INT REFERENCES \"{0}\".\"City\" (id) ON DELETE SET DEFAULT
        );
        ",
        api.schema_name()
    );

    api.raw_cmd(&sql);

    let schema = api.describe();

    schema.assert_table("User", |t| {
        t.assert_column("id", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_cascade", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_restrict", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_set_null", |c| c.assert_type_is_int_or_bigint())
            .assert_column("city_set_default", |c| c.assert_type_is_int_or_bigint())
            .assert_pk_on_columns(&["id"])
            .assert_foreign_key_on_columns(&["city"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::NoAction)
            })
            .assert_foreign_key_on_columns(&["city_cascade"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::Cascade)
            })
            .assert_foreign_key_on_columns(&["city_restrict"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::Restrict)
            })
            .assert_foreign_key_on_columns(&["city_set_default"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::SetDefault)
            })
            .assert_foreign_key_on_columns(&["city_set_null"], |fk| {
                fk.assert_references("City", &["id"])
                    .assert_on_delete(ForeignKeyAction::SetNull)
            })
    });
}

#[test_connector(tags(Postgres))]
fn postgres_enums_must_work(api: TestApi) {
    api.raw_cmd(&format!(
        "CREATE TYPE \"{}\".\"mood\" AS ENUM ('sad', 'ok', 'happy')",
        api.schema_name()
    ));
    let schema = api.describe();
    let got_enum = schema.walk(schema.find_enum("mood", None).expect("get enum"));
    let values = &["sad", "ok", "happy"];

    assert_eq!(got_enum.name(), "mood");
    let found_values = got_enum.values();
    assert_eq!(found_values.len(), values.len());
    for (a, b) in found_values.zip(values) {
        assert_eq!(a, *b)
    }
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn postgres_sequences_must_work(api: TestApi) {
    api.raw_cmd(&format!("CREATE SEQUENCE \"{}\".\"test\"", api.schema_name()));

    let schema = api.describe();
    let ext = extract_ext(&schema);
    let expected_ext = expect![[r#"
        PostgresSchemaExt {
            opclasses: [],
            indexes: [],
            expression_indexes: [],
            index_null_position: {},
            constraint_options: {},
            table_options: [],
            exclude_constraints: [],
            sequences: [
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "test",
                    start_value: 1,
                    min_value: 1,
                    max_value: 9223372036854775807,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 0,
                    virtual: false,
                },
            ],
            extensions: [
                DatabaseExtension {
                    name: "plpgsql",
                    schema: "pg_catalog",
                    version: "1.0",
                    relocatable: false,
                },
            ],
        }
    "#]];
    expected_ext.assert_debug_eq(&ext);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn postgres_multi_field_indexes_must_be_inferred_in_the_right_order(api: TestApi) {
    let schema = r#"
        CREATE TABLE "indexes_test" (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            age INTEGER NOT NULL
        );

        CREATE UNIQUE INDEX "my_idx" ON "indexes_test" (name, age);
        CREATE INDEX "my_idx2" ON "indexes_test" (age, name);
    "#;
    api.raw_cmd(schema);

    let schema = api.describe();
    let found: Vec<_> = schema
        .table_walkers()
        .next()
        .unwrap()
        .indexes()
        .map(|idx| (idx.name(), idx.is_unique(), idx.column_names().collect::<Vec<_>>()))
        .collect();

    let expectation = expect![[r#"
        [
            (
                "indexes_test_pkey",
                false,
                [
                    "id",
                ],
            ),
            (
                "my_idx",
                true,
                [
                    "name",
                    "age",
                ],
            ),
            (
                "my_idx2",
                false,
                [
                    "age",
                    "name",
                ],
            ),
        ]
    "#]];
    expectation.assert_debug_eq(&found);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn escaped_quotes_in_string_defaults_must_be_unescaped(api: TestApi) {
    let create_table = r#"
        CREATE TABLE "string_defaults_test" (
            id INTEGER PRIMARY KEY,
            regular VARCHAR NOT NULL DEFAULT E'meow, says the cat',
            escaped VARCHAR NOT NULL DEFAULT E'"That\'s a lot of fish!" - Godzilla, 1998',
            escaped_no_e VARCHAR NOT NULL DEFAULT '"That''s a lot of fish!" - Godzilla, 1998'
        );
    "#;

    api.raw_cmd(create_table);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {
                "public",
            },
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
                        name: "id",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        name: "regular",
                        tpe: ColumnType {
                            full_data_type: "varchar",
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
                        name: "escaped",
                        tpe: ColumnType {
                            full_data_type: "varchar",
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
                        name: "escaped_no_e",
                        tpe: ColumnType {
                            full_data_type: "varchar",
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
            ],
            foreign_keys: [],
            table_default_values: [
                (
                    TableColumnId(
                        1,
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
                        2,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "\"That's a lot of fish!\" - Godzilla, 1998",
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        3,
                    ),
                    DefaultValue {
                        kind: Value(
                            String(
                                "\"That's a lot of fish!\" - Godzilla, 1998",
                            ),
                        ),
                        constraint_name: None,
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
                    index_name: "string_defaults_test_pkey",
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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn seemingly_escaped_backslashes_in_string_literals_must_not_be_unescaped(api: TestApi) {
    // https://www.postgresql.org/docs/current/sql-syntax-lexical.html
    let create_table = r#"
        CREATE TABLE test (
            "model_name_space" VARCHAR(255) NOT NULL DEFAULT e'xyz\\Datasource\\Model'
        )
    "#;

    api.raw_cmd(create_table);
    let expectation = expect![[r#"
        SqlSchema {
            namespaces: {
                "public",
            },
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
                            full_data_type: "varchar",
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

#[test_connector(tags(Postgres))]
fn index_sort_order_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY,
            a  INT NOT NULL
        );

        CREATE INDEX foo ON A (a DESC);
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let index = table.indexes().nth(1).unwrap();
    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(1, columns.len());
    assert_eq!("a", columns[0].as_column().name());
    assert_eq!(Some(SQLSortOrder::Desc), columns[0].sort_order());
}

#[test_connector(tags(Postgres))]
fn index_sort_order_composite_type_desc_desc_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE A (
            a  INT NOT NULL,
            b  INT NOT NULL
        );

        CREATE INDEX foo ON A (a DESC, b DESC);
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
    assert_eq!(Some(SQLSortOrder::Desc), columns[1].sort_order());
}

#[test_connector(tags(Postgres))]
fn index_sort_order_composite_type_asc_desc_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE A (
            a  INT NOT NULL,
            b  INT NOT NULL
        );

        CREATE INDEX foo ON A (a ASC, b DESC);
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let index = table.indexes().next().unwrap();

    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(2, columns.len());

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Asc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Desc), columns[1].sort_order());
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn array_column_defaults(api: TestApi) {
    let schema = r#"
        CREATE TYPE "color" AS ENUM ('RED', 'GREEN', 'BLUE');

        CREATE TABLE "defaults" (
            text_empty TEXT[] NOT NULL DEFAULT '{}',
            text TEXT[] NOT NULL DEFAULT '{ ''abc'' }',
            text_c_escape TEXT[] NOT NULL DEFAULT E'{ \'abc\', \'def\' }',
            colors COLOR[] NOT NULL DEFAULT '{ RED, GREEN }',
            int_defaults INT4[] NOT NULL DEFAULT '{ 9, 12999, -4, 0, 1249849 }',
            float_defaults DOUBLE PRECISION[] NOT NULL DEFAULT '{ 0, 9.12, 3.14, 0.1242, 124949.124949 }',
            bool_defaults BOOLEAN[] NOT NULL DEFAULT '{ true, true, true, false }',
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT '{ ''2022-09-01T08:00Z'', "2021-09-01T08:00Z"}'
        );
    "#;

    api.raw_cmd(schema);
    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();

    let assert_default = |colname: &str, expected_default: Vec<PrismaValue>| {
        let col = table.column(colname).unwrap();
        let value = col.default().unwrap().as_value().unwrap();
        assert_eq!(value, &PrismaValue::List(expected_default));
    };

    assert_default("text_empty", vec![]);
    assert_default("text", vec![PrismaValue::String("abc".to_owned())]);
    assert_default("text_c_escape", vec!["abc".into(), "def".into()]);
    assert_default(
        "colors",
        vec![
            PrismaValue::Enum("RED".to_owned()),
            PrismaValue::Enum("GREEN".to_owned()),
        ],
    );
    assert_default(
        "int_defaults",
        vec![
            PrismaValue::Int(9),
            PrismaValue::Int(12999),
            PrismaValue::Int(-4),
            PrismaValue::Int(0),
            PrismaValue::Int(1249849),
        ],
    );
    assert_default(
        "float_defaults",
        vec![
            PrismaValue::Float("0.0".parse().unwrap()),
            PrismaValue::Float("9.12".parse().unwrap()),
            PrismaValue::Float("3.14".parse().unwrap()),
            PrismaValue::Float("0.1242".parse().unwrap()),
            PrismaValue::Float("124949.124949".parse().unwrap()),
        ],
    );
    assert_default(
        "bool_defaults",
        vec![
            PrismaValue::Boolean(true),
            PrismaValue::Boolean(true),
            PrismaValue::Boolean(true),
            PrismaValue::Boolean(false),
        ],
    );
    // assert_default(
    //     "datetime_defaults",
    //     vec![
    //         PrismaValue::Enum("2022-09-01 08:00:00+00".to_owned()),
    //         PrismaValue::Enum("2021-09-01 08:00:00+00".to_owned()),
    //     ],
    // );
}

#[test_connector(tags(Postgres))]
fn array_column_defaults_with_array_constructor_syntax(api: TestApi) {
    let schema = r#"
        CREATE TYPE "color" AS ENUM ('RED', 'GREEN', 'BLUE');

        CREATE TABLE "defaults" (
            text_empty TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[],
            text TEXT[] NOT NULL DEFAULT ARRAY['abc']::TEXT[],
            text_c_escape TEXT[] NOT NULL DEFAULT ARRAY[E'abc', E'def']::TEXT[],
            varchar_empty VARCHAR(255)[] NOT NULL DEFAULT ARRAY[]::VARCHAR(255)[],
            colors COLOR[] NOT NULL DEFAULT ARRAY['RED', 'GREEN']::COLOR[],
            int_defaults INT4[] NOT NULL DEFAULT ARRAY[9, 12999, -4, 0, 1249849]::INT4[],
            float_defaults DOUBLE PRECISION[] NOT NULL DEFAULT ARRAY[0, 9.12, 3.14, 0.1242, 124949.124949]::DOUBLE PRECISION[],
            bool_defaults BOOLEAN[] NOT NULL DEFAULT ARRAY[true, true, true, false]::BOOLEAN[],
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT ARRAY['2022-09-01T08:00Z','2021-09-01T08:00Z']::TIMESTAMP WITH TIME ZONE[]
        );
    "#;

    api.raw_cmd(schema);
    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();

    let assert_default = |colname: &str, expected_default: Vec<PrismaValue>| {
        let col = table.column(colname).unwrap();
        let value = col.default().unwrap().as_value().unwrap();
        assert_eq!(value, &PrismaValue::List(expected_default));
    };

    assert_default("text_empty", vec![]);
    assert_default("text", vec!["abc".into()]);
    assert_default("text_c_escape", vec!["abc".into(), "def".into()]);
    assert_default("varchar_empty", vec![]);
    assert_default(
        "colors",
        vec![
            PrismaValue::Enum("RED".to_owned()),
            PrismaValue::Enum("GREEN".to_owned()),
        ],
    );
    assert_default(
        "int_defaults",
        vec![
            PrismaValue::Int(9),
            PrismaValue::Int(12999),
            PrismaValue::Int(-4),
            PrismaValue::Int(0),
            PrismaValue::Int(1249849),
        ],
    );
    assert_default(
        "float_defaults",
        vec![
            PrismaValue::Float("0".parse().unwrap()),
            PrismaValue::Float("9.12".parse().unwrap()),
            PrismaValue::Float("3.14".parse().unwrap()),
            PrismaValue::Float("0.1242".parse().unwrap()),
            PrismaValue::Float("124949.124949".parse().unwrap()),
        ],
    );
    assert_default(
        "bool_defaults",
        vec![
            PrismaValue::Boolean(true),
            PrismaValue::Boolean(true),
            PrismaValue::Boolean(true),
            PrismaValue::Boolean(false),
        ],
    );
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn int_expressions_in_defaults(api: TestApi) {
    let schema = r#"
        CREATE TABLE "defaults" (
            mysum INT8 NOT NULL DEFAULT 5 + 32
        );
    "#;

    api.raw_cmd(schema);
    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let col = table.column("mysum").unwrap();
    let value = col.default().unwrap();
    assert!(value.is_db_generated());
}

#[test_connector(tags(Postgres14), exclude(CockroachDb))]
fn extensions_are_described_correctly(api: TestApi) {
    let schema = r#"CREATE EXTENSION IF NOT EXISTS citext;"#;

    api.raw_cmd(schema);

    let result = api.describe();
    let ext = extract_ext(&result);
    let expected_ext = expect![[r#"
        PostgresSchemaExt {
            opclasses: [],
            indexes: [],
            expression_indexes: [],
            index_null_position: {},
            constraint_options: {},
            table_options: [],
            exclude_constraints: [],
            sequences: [],
            extensions: [
                DatabaseExtension {
                    name: "citext",
                    schema: "public",
                    version: "1.6",
                    relocatable: true,
                },
                DatabaseExtension {
                    name: "plpgsql",
                    schema: "pg_catalog",
                    version: "1.0",
                    relocatable: false,
                },
            ],
        }
    "#]];

    expected_ext.assert_debug_eq(&ext);
}

// multi schema

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn multiple_schemas_with_same_table_names_are_described(api: TestApi) {
    let schema = r#"
           CREATE Schema "schema_0";
           CREATE TABLE "schema_0"."Table_0" ("id_0" SERIAL PRIMARY KEY, int Integer Default 0);

           CREATE Schema "schema_1";
           CREATE TABLE "schema_1"."Table_0" ("id_1" SERIAL PRIMARY KEY, int Integer Default 1);
    "#;

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
                            full_data_type: "int4",
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
                        0,
                    ),
                    Column {
                        name: "int",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                            full_data_type: "int4",
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
                        1,
                    ),
                    Column {
                        name: "int",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
            table_default_values: [
                (
                    TableColumnId(
                        0,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_0_id_0_seq",
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
                            Int(
                                0,
                            ),
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        2,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_0_id_1_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        3,
                    ),
                    DefaultValue {
                        kind: Value(
                            Int(
                                1,
                            ),
                        ),
                        constraint_name: None,
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
                    index_name: "Table_0_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "Table_0_pkey",
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

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn multiple_schemas_with_same_foreign_key_are_described(api: TestApi) {
    let schema = r#"
           CREATE Schema "schema_0";
           CREATE TABLE "schema_0"."Table_0" ("other" Integer, "id_0" SERIAL PRIMARY KEY);
           CREATE TABLE "schema_0"."Table_1" ("id_1" SERIAL PRIMARY KEY, o_id_0 Integer);
           ALTER TABLE "schema_0"."Table_1" ADD CONSTRAINT "fk_0" FOREIGN KEY ("o_id_0") REFERENCES "schema_0"."Table_0" ("id_0");

           CREATE Schema "schema_1";
           CREATE TABLE "schema_1"."Table_0" ("id_2" SERIAL PRIMARY KEY);
           CREATE TABLE "schema_1"."Table_1" ("id_3" SERIAL PRIMARY KEY, o_id_0 Integer);
           ALTER TABLE "schema_1"."Table_1" ADD CONSTRAINT "fk_0" FOREIGN KEY ("o_id_0") REFERENCES "schema_1"."Table_0" ("id_2");

           CREATE TABLE "schema_1"."Table_2" ("id_4" SERIAL PRIMARY KEY, o_id_0 Integer);
           ALTER TABLE "schema_1"."Table_2" ADD CONSTRAINT "fk_1" FOREIGN KEY ("o_id_0") REFERENCES "schema_0"."Table_0" ("id_0");
    "#;

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
                            full_data_type: "int4",
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
                            full_data_type: "int4",
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
                        1,
                    ),
                    Column {
                        name: "id_1",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        1,
                    ),
                    Column {
                        name: "o_id_0",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        2,
                    ),
                    Column {
                        name: "id_2",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                        name: "id_3",
                        tpe: ColumnType {
                            full_data_type: "int4",
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
                            full_data_type: "int4",
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
                            full_data_type: "int4",
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
                            full_data_type: "int4",
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
                        1,
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
                        3,
                    ),
                    referenced_table: TableId(
                        2,
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
            table_default_values: [
                (
                    TableColumnId(
                        1,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_0_id_0_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        2,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_1_id_1_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        4,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_0_id_2_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        5,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_1_id_3_seq",
                        ),
                        constraint_name: None,
                    },
                ),
                (
                    TableColumnId(
                        7,
                    ),
                    DefaultValue {
                        kind: Sequence(
                            "Table_2_id_4_seq",
                        ),
                        constraint_name: None,
                    },
                ),
            ],
            view_default_values: [],
            foreign_key_columns: [
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        0,
                    ),
                    constrained_column: TableColumnId(
                        3,
                    ),
                    referenced_column: TableColumnId(
                        1,
                    ),
                },
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        1,
                    ),
                    constrained_column: TableColumnId(
                        6,
                    ),
                    referenced_column: TableColumnId(
                        4,
                    ),
                },
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        2,
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
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "Table_1_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        2,
                    ),
                    index_name: "Table_0_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        3,
                    ),
                    index_name: "Table_1_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        4,
                    ),
                    index_name: "Table_2_pkey",
                    tpe: PrimaryKey,
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
                        4,
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
                        5,
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

#[test_connector(tags(Postgres12, Postgres13, Postgres14, Postgres15, Postgres16))]
fn multiple_schemas_are_described(api: TestApi) {
    let schema = r#"
           CREATE Schema "schema_0";
           CREATE TABLE "schema_0"."Table_0" ("id_0" SERIAL PRIMARY KEY);
           CREATE TABLE "schema_0"."Table_1" ("id_1" SERIAL PRIMARY KEY, o_id_0 Integer References "schema_0"."Table_0"("id_0"));
           CREATE SEQUENCE "schema_0"."Sequence_0" START 1;
           CREATE TYPE "schema_0"."Type_0" AS ENUM ('happy');
           CREATE INDEX "Index_0" ON "schema_0"."Table_1"("o_id_0");
           CREATE VIEW "schema_0"."View_0" AS SELECT 0;
           CREATE PROCEDURE "schema_0"."Procedure_0" ()
            LANGUAGE SQL
            AS $$
            Select 0;
            $$;

           CREATE Schema "schema_1";
           CREATE TABLE "schema_1"."Table_2" ("id_2" SERIAL PRIMARY KEY);
           CREATE TABLE "schema_1"."Table_3" ("id_3" SERIAL PRIMARY KEY, o_id_2 Integer References "schema_1"."Table_2"("id_2"));
           CREATE SEQUENCE "schema_1"."Sequence_1" START 100;
           CREATE TYPE "schema_1"."Type_1" AS ENUM ('happy');
           CREATE INDEX "Index_1" ON "schema_1"."Table_3"("o_id_2");
           CREATE VIEW "schema_1"."View_1" AS SELECT 1;
           CREATE PROCEDURE "schema_1"."Procedure_1" ()
            LANGUAGE SQL
            AS $$
            Select 1;
            $$;
    "#;

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
                },
                Table {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Table_1",
                },
                Table {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Table_2",
                },
                Table {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Table_3",
                },
            ],
            enums: [
                Enum {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Type_0",
                    values: [
                        "happy",
                    ],
                },
                Enum {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Type_1",
                    values: [
                        "happy",
                    ],
                },
            ],
            columns: [
                (
                    TableId(
                        0,
                    ),
                    Column {
                        name: "id_0",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String("Integer"),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "Table_0_id_0_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "id_1",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String("Integer"),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "Table_1_id_1_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        1,
                    ),
                    Column {
                        name: "o_id_0",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                String("Integer"),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
                (
                    TableId(
                        2,
                    ),
                    Column {
                        name: "id_2",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String("Integer"),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "Table_2_id_2_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        3,
                    ),
                    Column {
                        name: "id_3",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String("Integer"),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "Table_3_id_3_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
                    },
                ),
                (
                    TableId(
                        3,
                    ),
                    Column {
                        name: "o_id_2",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Nullable,
                            native_type: Some(
                                String("Integer"),
                            ),
                        },
                        default: None,
                        auto_increment: false,
                    },
                ),
            ],
            foreign_keys: [
                ForeignKey {
                    constrained_table: TableId(
                        1,
                    ),
                    referenced_table: TableId(
                        0,
                    ),
                    constraint_name: Some(
                        "Table_1_o_id_0_fkey",
                    ),
                    on_delete_action: NoAction,
                    on_update_action: NoAction,
                },
                ForeignKey {
                    constrained_table: TableId(
                        3,
                    ),
                    referenced_table: TableId(
                        2,
                    ),
                    constraint_name: Some(
                        "Table_3_o_id_2_fkey",
                    ),
                    on_delete_action: NoAction,
                    on_update_action: NoAction,
                },
            ],
            foreign_key_columns: [
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        0,
                    ),
                    constrained_column: ColumnId(
                        2,
                    ),
                    referenced_column: ColumnId(
                        0,
                    ),
                },
                ForeignKeyColumn {
                    foreign_key_id: ForeignKeyId(
                        1,
                    ),
                    constrained_column: ColumnId(
                        5,
                    ),
                    referenced_column: ColumnId(
                        3,
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
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "Index_0",
                    tpe: Normal,
                },
                Index {
                    table_id: TableId(
                        1,
                    ),
                    index_name: "Table_1_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        2,
                    ),
                    index_name: "Table_2_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        3,
                    ),
                    index_name: "Index_1",
                    tpe: Normal,
                },
                Index {
                    table_id: TableId(
                        3,
                    ),
                    index_name: "Table_3_pkey",
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
                IndexColumn {
                    index_id: IndexId(
                        1,
                    ),
                    column_id: ColumnId(
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
                    column_id: ColumnId(
                        1,
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
                    column_id: ColumnId(
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
                    column_id: ColumnId(
                        5,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
                IndexColumn {
                    index_id: IndexId(
                        5,
                    ),
                    column_id: ColumnId(
                        4,
                    ),
                    sort_order: Some(
                        Asc,
                    ),
                    length: None,
                },
            ],
            views: [
                View {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "View_0",
                    definition: Some(
                        " SELECT 0;",
                    ),
                },
                View {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "View_1",
                    definition: Some(
                        " SELECT 1;",
                    ),
                },
            ],
            procedures: [
                Procedure {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "Procedure_0",
                    definition: Some(
                        "CREATE OR REPLACE PROCEDURE schema_0.\"Procedure_0\"()\n LANGUAGE sql\nAS $procedure$\n            Select 0;\n            $procedure$\n",
                    ),
                },
                Procedure {
                    namespace_id: NamespaceId(
                        1,
                    ),
                    name: "Procedure_1",
                    definition: Some(
                        "CREATE OR REPLACE PROCEDURE schema_1.\"Procedure_1\"()\n LANGUAGE sql\nAS $procedure$\n            Select 1;\n            $procedure$\n",
                    ),
                },
            ],
            user_defined_types: [],
            connector_data: <ConnectorData>,
            runtime_namespace: None,
        }
    "#]];

    expected_schema.assert_debug_eq(&schema);
}

fn extract_ext(schema: &SqlSchema) -> &PostgresSchemaExt {
    schema.downcast_connector_data()
}

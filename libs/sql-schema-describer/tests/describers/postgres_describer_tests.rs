mod cockroach_describer_tests;

use crate::test_api::*;
use barrel::{types, Migration};
use pretty_assertions::assert_eq;
use prisma_value::PrismaValue;
use sql_schema_describer::{postgres::PostgresSchemaExt, *};

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
    let mut migration = Migration::new().schema(api.schema_name());
    migration.create_table("User", move |t| {
        t.add_column("array_bin_col", types::array(&types::binary()));
        t.add_column("array_bool_col", types::array(&types::boolean()));
        t.add_column("array_date_col", types::array(&types::date()));
        t.add_column("array_double_col", types::array(&types::double()));
        t.add_column("array_float_col", types::array(&types::float()));
        t.add_column("array_int_col", types::array(&types::integer()));
        t.add_column("array_text_col", types::array(&types::text()));
        t.add_column("array_varchar_col", types::array(&types::varchar(255)));
        t.add_column("bigint_col", types::custom("BIGINT"));
        t.add_column("bigserial_col", types::custom("BIGSERIAL"));
        t.add_column("bit_col", types::custom("BIT"));
        t.add_column("bit_varying_col", types::custom("BIT VARYING(1)"));
        t.add_column("binary_col", types::binary());
        t.add_column("boolean_col", types::boolean());
        t.add_column("box_col", types::custom("BOX"));
        t.add_column("char_col", types::custom("CHARACTER(1)"));
        t.add_column("circle_col", types::custom("CIRCLE"));
        t.add_column("date_time_col", types::date());
        t.add_column("double_col", types::double());
        t.add_column("float_col", types::float());
        t.add_column("int_col", types::integer());
        t.add_column("line_col", types::custom("LINE"));
        t.add_column("lseg_col", types::custom("LSEG"));
        t.add_column("numeric_col", types::custom("NUMERIC"));
        t.add_column("path_col", types::custom("PATH"));
        t.add_column("pg_lsn_col", types::custom("PG_LSN"));
        t.add_column("polygon_col", types::custom("POLYGON"));
        t.add_column("smallint_col", types::custom("SMALLINT"));
        t.add_column("smallserial_col", types::custom("SMALLSERIAL"));
        t.add_column("serial_col", types::custom("SERIAL"));
        t.add_column("primary_col", types::primary());
        t.add_column("string1_col", types::text());
        t.add_column("string2_col", types::varchar(1));
        t.add_column("time_col", types::custom("TIME"));
        t.add_column("timetz_col", types::custom("TIMETZ"));
        t.add_column("timestamp_col", types::custom("TIMESTAMP"));
        t.add_column("timestamptz_col", types::custom("TIMESTAMPTZ"));
        t.add_column("tsquery_col", types::custom("TSQUERY"));
        t.add_column("tsvector_col", types::custom("TSVECTOR"));
        t.add_column("txid_col", types::custom("TXID_SNAPSHOT"));
        t.add_column("json_col", types::json());
        t.add_column("jsonb_col", types::custom("JSONB"));
        t.add_column("uuid_col", types::uuid());
    });

    let full_sql = migration.make::<barrel::backend::Pg>();
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
                        name: "array_bin_col",
                        tpe: ColumnType {
                            full_data_type: "_bytea",
                            family: Binary,
                            arity: List,
                            native_type: Some(
                                String(
                                    "ByteA",
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
                        name: "array_bool_col",
                        tpe: ColumnType {
                            full_data_type: "_bool",
                            family: Boolean,
                            arity: List,
                            native_type: Some(
                                String(
                                    "Boolean",
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
                        name: "array_date_col",
                        tpe: ColumnType {
                            full_data_type: "_date",
                            family: DateTime,
                            arity: List,
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
                        name: "array_double_col",
                        tpe: ColumnType {
                            full_data_type: "_float8",
                            family: Float,
                            arity: List,
                            native_type: Some(
                                String(
                                    "DoublePrecision",
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
                        name: "array_float_col",
                        tpe: ColumnType {
                            full_data_type: "_float8",
                            family: Float,
                            arity: List,
                            native_type: Some(
                                String(
                                    "DoublePrecision",
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
                        name: "array_int_col",
                        tpe: ColumnType {
                            full_data_type: "_int4",
                            family: Int,
                            arity: List,
                            native_type: Some(
                                String(
                                    "Integer",
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
                        name: "array_text_col",
                        tpe: ColumnType {
                            full_data_type: "_text",
                            family: String,
                            arity: List,
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
                        name: "array_varchar_col",
                        tpe: ColumnType {
                            full_data_type: "_varchar",
                            family: String,
                            arity: List,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
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
                        name: "bigint_col",
                        tpe: ColumnType {
                            full_data_type: "int8",
                            family: BigInt,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "BigInt",
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
                        name: "bigserial_col",
                        tpe: ColumnType {
                            full_data_type: "int8",
                            family: BigInt,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "BigInt",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "User_bigserial_col_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
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
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Bit": Number(
                                        1,
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
                        name: "bit_varying_col",
                        tpe: ColumnType {
                            full_data_type: "varbit",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarBit": Number(
                                        1,
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
                        name: "binary_col",
                        tpe: ColumnType {
                            full_data_type: "bytea",
                            family: Binary,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "ByteA",
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
                        name: "boolean_col",
                        tpe: ColumnType {
                            full_data_type: "bool",
                            family: Boolean,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Boolean",
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
                        name: "box_col",
                        tpe: ColumnType {
                            full_data_type: "box",
                            family: Unsupported(
                                "box",
                            ),
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
                        name: "char_col",
                        tpe: ColumnType {
                            full_data_type: "bpchar",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Char": Number(
                                        1,
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
                        name: "circle_col",
                        tpe: ColumnType {
                            full_data_type: "circle",
                            family: Unsupported(
                                "circle",
                            ),
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
                        name: "date_time_col",
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
                        name: "double_col",
                        tpe: ColumnType {
                            full_data_type: "float8",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "DoublePrecision",
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
                            full_data_type: "float8",
                            family: Float,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "DoublePrecision",
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
                        name: "int_col",
                        tpe: ColumnType {
                            full_data_type: "int4",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Integer",
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
                        name: "line_col",
                        tpe: ColumnType {
                            full_data_type: "line",
                            family: Unsupported(
                                "line",
                            ),
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
                        name: "lseg_col",
                        tpe: ColumnType {
                            full_data_type: "lseg",
                            family: Unsupported(
                                "lseg",
                            ),
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
                        name: "numeric_col",
                        tpe: ColumnType {
                            full_data_type: "numeric",
                            family: Decimal,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Decimal": Null,
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
                        name: "path_col",
                        tpe: ColumnType {
                            full_data_type: "path",
                            family: Unsupported(
                                "path",
                            ),
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
                        name: "pg_lsn_col",
                        tpe: ColumnType {
                            full_data_type: "pg_lsn",
                            family: Unsupported(
                                "pg_lsn",
                            ),
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
                        name: "polygon_col",
                        tpe: ColumnType {
                            full_data_type: "polygon",
                            family: Unsupported(
                                "polygon",
                            ),
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
                        name: "smallint_col",
                        tpe: ColumnType {
                            full_data_type: "int2",
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
                        name: "smallserial_col",
                        tpe: ColumnType {
                            full_data_type: "int2",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "SmallInt",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "User_smallserial_col_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
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
                                String(
                                    "Integer",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "User_serial_col_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
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
                                String(
                                    "Integer",
                                ),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Sequence(
                                    "User_primary_col_seq",
                                ),
                                constraint_name: None,
                            },
                        ),
                        auto_increment: true,
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
                        name: "string2_col",
                        tpe: ColumnType {
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        1,
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
                        name: "time_col",
                        tpe: ColumnType {
                            full_data_type: "time",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Time": Number(
                                        6,
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
                        name: "timetz_col",
                        tpe: ColumnType {
                            full_data_type: "timetz",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Timetz": Number(
                                        6,
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
                        name: "timestamp_col",
                        tpe: ColumnType {
                            full_data_type: "timestamp",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Timestamp": Number(
                                        6,
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
                        name: "timestamptz_col",
                        tpe: ColumnType {
                            full_data_type: "timestamptz",
                            family: DateTime,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "Timestamptz": Number(
                                        6,
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
                        name: "tsquery_col",
                        tpe: ColumnType {
                            full_data_type: "tsquery",
                            family: Unsupported(
                                "tsquery",
                            ),
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
                        name: "tsvector_col",
                        tpe: ColumnType {
                            full_data_type: "tsvector",
                            family: Unsupported(
                                "tsvector",
                            ),
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
                        name: "txid_col",
                        tpe: ColumnType {
                            full_data_type: "txid_snapshot",
                            family: Unsupported(
                                "txid_snapshot",
                            ),
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
                        name: "json_col",
                        tpe: ColumnType {
                            full_data_type: "json",
                            family: Json,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Json",
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
                        name: "jsonb_col",
                        tpe: ColumnType {
                            full_data_type: "jsonb",
                            family: Json,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "JsonB",
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
                        name: "uuid_col",
                        tpe: ColumnType {
                            full_data_type: "uuid",
                            family: Uuid,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Uuid",
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
                    index_name: "User_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "User_uuid_col_key",
                    tpe: Unique,
                },
            ],
            index_columns: [
                IndexColumn {
                    index_id: IndexId(
                        0,
                    ),
                    column_id: ColumnId(
                        30,
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
                        42,
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

    if api.connector_tags().contains(Tags::Postgres9) {
        return; // sequence max values work differently on postgres 9
    }

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
                (
                    IndexId(
                        1,
                    ),
                    BTree,
                ),
            ],
            sequences: [
                Sequence {
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
        }
    "#]];
    expected_ext.assert_debug_eq(&ext);
}

#[test_connector(tags(Postgres))]
fn cross_schema_references_are_not_allowed(api: TestApi) {
    let schema2 = format!("{}_2", api.schema_name());

    let sql = format!(
        "DROP SCHEMA IF EXISTS \"{0}\" CASCADE;
         CREATE SCHEMA \"{0}\";
         CREATE TABLE \"{0}\".\"City\" (id INT PRIMARY KEY);
         CREATE TABLE \"User\" (
            id INT PRIMARY KEY,
            city INT REFERENCES \"{0}\".\"City\" (id) ON DELETE NO ACTION
        );
        ",
        schema2,
    );

    api.raw_cmd(&sql);

    let err = api.describe_error();
    let fk_name = "User_city_fkey";

    assert_eq!(
        format!("Illegal cross schema reference from `prisma-tests.User` to `prisma-tests_2.City` in constraint `{}`. Foreign keys between database schemas are not supported in Prisma. Please follow the GitHub ticket: https://github.com/prisma/prisma/issues/1175", fk_name),
        err.to_string()
    );
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
    let got_enum = schema.get_enum("mood").expect("get enum");
    let values = &["sad", "ok", "happy"];

    assert_eq!(got_enum.name, "mood");
    assert_eq!(got_enum.values, values);
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
            sequences: [
                Sequence {
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
        }
    "#]];
    expected_ext.assert_debug_eq(&ext);
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn postgres_multi_field_indexes_must_be_inferred_in_the_right_order(api: TestApi) {
    let schema = r##"
        CREATE TABLE "indexes_test" (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            age INTEGER NOT NULL
        );

        CREATE UNIQUE INDEX "my_idx" ON "indexes_test" (name, age);
        CREATE INDEX "my_idx2" ON "indexes_test" (age, name);
    "##;
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
            tables: [
                Table {
                    name: "string_defaults_test",
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
                            full_data_type: "int4",
                            family: Int,
                            arity: Required,
                            native_type: Some(
                                String(
                                    "Integer",
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
                        name: "regular",
                        tpe: ColumnType {
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Null,
                                }),
                            ),
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
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Null,
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "\"That's a lot of fish!\" - Godzilla, 1998",
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
                        name: "escaped_no_e",
                        tpe: ColumnType {
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Null,
                                }),
                            ),
                        },
                        default: Some(
                            DefaultValue {
                                kind: Value(
                                    String(
                                        "\"That's a lot of fish!\" - Godzilla, 1998",
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
            tables: [
                Table {
                    name: "test",
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
                            full_data_type: "varchar",
                            family: String,
                            arity: Required,
                            native_type: Some(
                                Object({
                                    "VarChar": Number(
                                        255,
                                    ),
                                }),
                            ),
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
            foreign_key_columns: [],
            indexes: [],
            index_columns: [],
            views: [],
            procedures: [],
            user_defined_types: [],
            connector_data: <ConnectorData>,
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

fn extract_ext(schema: &SqlSchema) -> &PostgresSchemaExt {
    schema.downcast_connector_data()
}

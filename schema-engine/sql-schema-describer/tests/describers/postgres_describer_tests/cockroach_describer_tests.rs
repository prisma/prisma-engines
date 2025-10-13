use crate::test_api::*;
use prisma_value::PrismaValue;
use sql_schema_describer::{ColumnTypeFamily, postgres::PostgresSchemaExt};

#[test_connector(tags(CockroachDb))]
fn views_can_be_described(api: TestApi) {
    let full_sql = r#"
        CREATE TABLE a (a_id int);
        CREATE TABLE b (b_id int);
        CREATE VIEW ab AS SELECT a_id FROM a UNION ALL SELECT b_id FROM b;
    "#;

    api.raw_cmd(full_sql);
    let result = api.describe();
    let view = result.get_view("ab").expect("couldn't get ab view").to_owned();

    let expected_sql = "SELECT a_id FROM views_can_be_described.\"public\".a UNION ALL SELECT b_id FROM views_can_be_described.\"public\".b";

    assert_eq!("ab", &view.name);
    assert_eq!(expected_sql, view.definition.unwrap());
}

#[test_connector(tags(CockroachDb))]
fn all_cockroach_column_types_must_work(api: TestApi) {
    let migration = r#"
        CREATE TYPE "mood" AS ENUM ('sad', 'ok', 'happy');

        CREATE TABLE "User" (
            array_bin_col BYTEA[],
            array_bool_col BOOLEAN[],
            array_date_col DATE[],
            array_double_col DOUBLE PRECISION[],
            array_float_col FLOAT[],
            array_int_col INT[],
            array_text_col TEXT[],
            array_varchar_col VARCHAR(255)[],
            bigint_col BIGINT,
            bigserial_col BIGSERIAL,
            bit_col BIT,
            bit_varying_col BIT VARYING(1),
            binary_col BYTEA,
            boolean_col BOOLEAN,
            char_col CHARACTER(1),
            date_col DATE,
            date_time_col TIMESTAMP,
            double_col DOUBLE PRECISION,
            enum_col mood,
            float_col FLOAT,
            int_col INT,
            numeric_col NUMERIC,
            oid_col OID,
            smallint_col SMALLINT,
            smallserial_col SMALLSERIAL,
            serial_col SERIAL,
            string1_col TEXT,
            string2_col VARCHAR(1),
            time_col TIME,
            timetz_col TIMETZ,
            timestamp_col TIMESTAMP,
            timestamptz_col TIMESTAMP WITH TIME ZONE,
            json_col JSON,
            jsonb_col JSONB,
            uuid_col  UUID
        )
        "#;

    api.raw_cmd(migration);

    api.describe().assert_table("User", |t| {
        t.assert_column("array_bin_col", |c| {
            c.assert_full_data_type("_bytea")
                .assert_column_type_family(ColumnTypeFamily::Binary)
                .assert_is_list()
        })
        .assert_column("array_bool_col", |c| {
            c.assert_full_data_type("_bool")
                .assert_column_type_family(ColumnTypeFamily::Boolean)
                .assert_is_list()
        })
        .assert_column("array_date_col", |c| {
            c.assert_full_data_type("_date")
                .assert_column_type_family(ColumnTypeFamily::DateTime)
                .assert_is_list()
        })
        .assert_column("array_double_col", |c| {
            c.assert_full_data_type("_float8")
                .assert_column_type_family(ColumnTypeFamily::Float)
                .assert_is_list()
        })
        .assert_column("array_float_col", |c| {
            c.assert_full_data_type("_float8")
                .assert_column_type_family(ColumnTypeFamily::Float)
                .assert_is_list()
        })
        .assert_column("array_int_col", |c| {
            c.assert_full_data_type("_int4")
                .assert_column_type_family(ColumnTypeFamily::Int)
                .assert_is_list()
        })
        .assert_column("array_text_col", |c| {
            c.assert_full_data_type("_text")
                .assert_column_type_family(ColumnTypeFamily::String)
                .assert_is_list()
        })
        .assert_column("array_varchar_col", |c| {
            c.assert_full_data_type("_varchar")
                .assert_column_type_family(ColumnTypeFamily::String)
                .assert_is_list()
        })
        .assert_column("binary_col", |c| {
            c.assert_full_data_type("bytea")
                .assert_column_type_family(ColumnTypeFamily::Binary)
        })
        .assert_column("boolean_col", |c| {
            c.assert_full_data_type("bool")
                .assert_column_type_family(ColumnTypeFamily::Boolean)
        })
        .assert_column("date_col", |c| {
            c.assert_full_data_type("date")
                .assert_column_type_family(ColumnTypeFamily::DateTime)
        })
        .assert_column("double_col", |c| {
            c.assert_full_data_type("float8")
                .assert_column_type_family(ColumnTypeFamily::Float)
        })
        .assert_column("float_col", |c| {
            c.assert_full_data_type("float8")
                .assert_column_type_family(ColumnTypeFamily::Float)
        })
        .assert_column("int_col", |c| {
            c.assert_full_data_type("int4")
                .assert_column_type_family(ColumnTypeFamily::Int)
        })
        .assert_column("string1_col", |c| {
            c.assert_full_data_type("text")
                .assert_column_type_family(ColumnTypeFamily::String)
        })
        .assert_column("string2_col", |c| {
            c.assert_full_data_type("varchar")
                .assert_column_type_family(ColumnTypeFamily::String)
        })
        .assert_column("bigint_col", |c| {
            c.assert_full_data_type("int8")
                .assert_column_type_family(ColumnTypeFamily::BigInt)
        })
        .assert_column("bigserial_col", |c| {
            c.assert_full_data_type("int8")
                .assert_column_type_family(ColumnTypeFamily::BigInt)
        })
        .assert_column("bit_col", |c| {
            c.assert_full_data_type("bit")
                .assert_column_type_family(ColumnTypeFamily::String)
        })
        .assert_column("bit_varying_col", |c| {
            c.assert_full_data_type("varbit")
                .assert_column_type_family(ColumnTypeFamily::String)
        })
        .assert_column("char_col", |c| {
            c.assert_full_data_type("bpchar")
                .assert_column_type_family(ColumnTypeFamily::String)
        })
        .assert_column("oid_col", |c| {
            c.assert_full_data_type("oid")
                .assert_column_type_family(ColumnTypeFamily::Int)
        })
        .assert_column("time_col", |c| {
            c.assert_full_data_type("time")
                .assert_column_type_family(ColumnTypeFamily::DateTime)
        })
        .assert_column("timetz_col", |c| {
            c.assert_full_data_type("timetz")
                .assert_column_type_family(ColumnTypeFamily::DateTime)
        })
        .assert_column("timestamp_col", |c| {
            c.assert_full_data_type("timestamp")
                .assert_column_type_family(ColumnTypeFamily::DateTime)
        })
        .assert_column("timestamptz_col", |c| {
            c.assert_full_data_type("timestamptz")
                .assert_column_type_family(ColumnTypeFamily::DateTime)
        })
        .assert_column("numeric_col", |c| {
            c.assert_full_data_type("numeric")
                .assert_column_type_family(ColumnTypeFamily::Decimal)
        })
        .assert_column("smallint_col", |c| {
            c.assert_full_data_type("int2")
                .assert_column_type_family(ColumnTypeFamily::Int)
        })
        .assert_column("smallserial_col", |c| {
            c.assert_full_data_type("int2")
                .assert_column_type_family(ColumnTypeFamily::Int)
        })
        .assert_column("serial_col", |c| {
            c.assert_full_data_type("int4")
                .assert_column_type_family(ColumnTypeFamily::Int)
        })
        .assert_column("json_col", |c| {
            c.assert_full_data_type("jsonb")
                .assert_column_type_family(ColumnTypeFamily::Json)
        })
        .assert_column("jsonb_col", |c| {
            c.assert_full_data_type("jsonb")
                .assert_column_type_family(ColumnTypeFamily::Json)
        })
        .assert_column("uuid_col", |c| {
            c.assert_full_data_type("uuid")
                .assert_column_type_family(ColumnTypeFamily::Uuid)
        })
    });
}

#[test_connector(tags(CockroachDb))]
fn multi_field_indexes_must_be_inferred_in_the_right_order(api: TestApi) {
    let schema = format!(
        r##"
            CREATE TABLE "{schema_name}"."indexes_test" (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                age INTEGER NOT NULL
            );

            CREATE UNIQUE INDEX "my_idx" ON "{schema_name}"."indexes_test" (name, age);
            CREATE INDEX "my_idx2" ON "{schema_name}"."indexes_test" (age, name);
        "##,
        schema_name = api.schema_name()
    );
    api.raw_cmd(&schema);
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
                    name: "indexes_test",
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
                            full_data_type: "text",
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
                        name: "name",
                        tpe: ColumnType {
                            full_data_type: "text",
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
                        name: "age",
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
                    index_name: "indexes_test_pkey",
                    tpe: PrimaryKey,
                },
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "my_idx",
                    tpe: Unique,
                },
                Index {
                    table_id: TableId(
                        0,
                    ),
                    index_name: "my_idx2",
                    tpe: Normal,
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
                        1,
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

#[test_connector(tags(CockroachDb))]
fn escaped_characters_in_string_defaults(api: TestApi) {
    // https://www.postgresql.org/docs/current/sql-syntax-lexical.html
    let init = r#"
        CREATE TABLE "Fruit" (
            id              SERIAL PRIMARY KEY,
            seasonality     TEXT DEFAULT '"summer"',
            contains        TEXT DEFAULT '''potassium''',
            "sideNames"     TEXT DEFAULT E'top\ndown'
        );
    "#;
    api.raw_cmd(init);
    let schema = api.describe();
    let table = schema.table_walker("Fruit").unwrap();

    let expect_col = |name: &str, expected: &str| {
        let col = table.column(name).unwrap();
        let default = col.default().unwrap().as_value().unwrap().as_string().unwrap();
        assert_eq!(default, expected);
    };
    expect_col("seasonality", r#""summer""#);
    expect_col("contains", r#"'potassium'"#);
    expect_col("sideNames", "top\ndown");
}

#[test_connector(tags(CockroachDb221))]
fn cockroachdb_22_1_sequences_must_work(api: TestApi) {
    // https://www.cockroachlabs.com/docs/v21.2/create-sequence.html
    let sql = r#"
        -- Defaults
        CREATE SEQUENCE "test";

        -- Not cycling. All crdb sequences are like that.
        CREATE SEQUENCE "testnotcycling" NO CYCLE;

        -- Other options
        CREATE SEQUENCE "testmore"
            INCREMENT 4
            MINVALUE 10
            MAXVALUE 100
            START 20
            CACHE 7;
    "#;
    api.raw_cmd(sql);

    let schema = api.describe();
    let ext: &PostgresSchemaExt = schema.downcast_connector_data();
    let expected_ext = expect![[r#"
        PostgresSchemaExt {
            opclasses: [],
            indexes: [],
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
                    cache_size: 1,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "testmore",
                    start_value: 20,
                    min_value: 10,
                    max_value: 100,
                    increment_by: 4,
                    cycle: false,
                    cache_size: 7,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "testnotcycling",
                    start_value: 1,
                    min_value: 1,
                    max_value: 9223372036854775807,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 1,
                    virtual: false,
                },
            ],
            extensions: [],
        }
    "#]];
    expected_ext.assert_debug_eq(&ext);
}

#[test_connector(tags(CockroachDb222))]
fn cockroachdb_22_2_sequences_must_work(api: TestApi) {
    // https://www.cockroachlabs.com/docs/v21.2/create-sequence.html
    let sql = r#"
        -- Defaults
        CREATE SEQUENCE "test";

        -- Not cycling. All crdb sequences are like that.
        CREATE SEQUENCE "testnotcycling" NO CYCLE;

        -- Other options
        CREATE SEQUENCE "testmore"
            INCREMENT 4
            MINVALUE 10
            MAXVALUE 100
            START 20
            CACHE 7;
    "#;
    api.raw_cmd(sql);

    let schema = api.describe();
    let ext: &PostgresSchemaExt = schema.downcast_connector_data();
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
                    max_value: 2147483647,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 1,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "testmore",
                    start_value: 20,
                    min_value: 10,
                    max_value: 100,
                    increment_by: 4,
                    cycle: false,
                    cache_size: 7,
                    virtual: false,
                },
                Sequence {
                    namespace_id: NamespaceId(
                        0,
                    ),
                    name: "testnotcycling",
                    start_value: 1,
                    min_value: 1,
                    max_value: 2147483647,
                    increment_by: 1,
                    cycle: false,
                    cache_size: 1,
                    virtual: false,
                },
            ],
            extensions: [],
        }
    "#]];
    expected_ext.assert_debug_eq(&ext);
}

#[test_connector(tags(CockroachDb))]
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
    let value = col.default().unwrap().as_value().unwrap();
    assert!(matches!(value, PrismaValue::Int(37)));
}

#[test_connector(tags(CockroachDb))]
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
            datetime_defaults TIMESTAMPTZ[] NOT NULL DEFAULT '{ "2022-09-01T08:00Z","2021-09-01T08:00Z"}'
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
}

use crate::test_api::*;
use sql_schema_describer::{ColumnTypeFamily, IndexColumn, SQLSortOrder};

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

    let expected_sql = "SELECT a_id FROM views_can_be_described.\"prisma-tests\".a UNION ALL SELECT b_id FROM views_can_be_described.\"prisma-tests\".b";

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
        .assert_column("enum_col", |c| {
            c.assert_full_data_type("mood")
                .assert_column_type_family(ColumnTypeFamily::Enum("mood".into()))
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
fn cockroach_multi_field_indexes_must_be_inferred_in_the_right_order(api: TestApi) {
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

    let schema = api.describe();

    let table = schema.table_bang("indexes_test");
    let index = &table.indices[1];

    assert_eq!(
        &index.columns,
        &[
            IndexColumn {
                name: "name".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None
            },
            IndexColumn {
                name: "age".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None
            },
        ]
    );
    assert!(index.tpe.is_unique());

    let index = &table.indices[0];

    assert!(!index.tpe.is_unique());
    assert_eq!(
        &index.columns,
        &[
            IndexColumn {
                name: "age".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None
            },
            IndexColumn {
                name: "name".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None
            },
        ]
    );
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
    let table = schema.table_bang("Fruit");

    let expect_col = |name: &str, expected: &str| {
        let col = table.column_bang(name);
        let default = col.default.as_ref().unwrap().as_value().unwrap().as_string().unwrap();
        assert_eq!(default, expected);
    };
    expect_col("seasonality", r#""summer""#);
    expect_col("contains", r#"'potassium'"#);
    expect_col("sideNames", "top\ndown");
}

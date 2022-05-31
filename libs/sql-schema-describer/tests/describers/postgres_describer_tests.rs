mod cockroach_describer_tests;

use crate::test_api::*;
use barrel::{types, Migration};
use indoc::indoc;
use native_types::{NativeType, PostgresType};
use pretty_assertions::assert_eq;
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
        // TODO: Test also autoincrement variety
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
    let mut result = api.describe();
    let table = result.tables.iter_mut().find(|t| t.name == "User").unwrap();
    // Ensure columns are sorted as expected when comparing
    table.columns.sort_unstable_by(|a, b| a.name.cmp(&b.name));
    let mut expected_columns = vec![
        Column {
            name: "array_bin_col".into(),
            tpe: ColumnType {
                full_data_type: "_bytea".into(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::ByteA.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_bool_col".into(),
            tpe: ColumnType {
                full_data_type: "_bool".into(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::Boolean.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_date_col".into(),
            tpe: ColumnType {
                full_data_type: "_date".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::Date.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_double_col".into(),
            tpe: ColumnType {
                full_data_type: "_float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::DoublePrecision.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_float_col".into(),
            tpe: ColumnType {
                full_data_type: "_float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::DoublePrecision.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_int_col".into(),
            tpe: ColumnType {
                full_data_type: "_int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::Integer.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_text_col".into(),
            tpe: ColumnType {
                full_data_type: "_text".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::Text.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "array_varchar_col".into(),
            tpe: ColumnType {
                full_data_type: "_varchar".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::List,
                native_type: Some(PostgresType::VarChar(Some(255)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "binary_col".into(),
            tpe: ColumnType {
                full_data_type: "bytea".into(),
                family: ColumnTypeFamily::Binary,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::ByteA.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "boolean_col".into(),
            tpe: ColumnType {
                full_data_type: "bool".into(),
                family: ColumnTypeFamily::Boolean,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Boolean.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "date_time_col".into(),
            tpe: ColumnType {
                full_data_type: "date".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Date.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "double_col".into(),
            tpe: ColumnType {
                full_data_type: "float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::DoublePrecision.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "float_col".into(),
            tpe: ColumnType {
                full_data_type: "float8".into(),
                family: ColumnTypeFamily::Float,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::DoublePrecision.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "int_col".into(),
            tpe: ColumnType {
                full_data_type: "int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Integer.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "primary_col".into(),
            tpe: ColumnType {
                full_data_type: "int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Integer.to_json()),
            },
            default: Some(DefaultValue::sequence("User_primary_col_seq".to_string())),
            auto_increment: true,
        },
        Column {
            name: "string1_col".into(),
            tpe: ColumnType {
                full_data_type: "text".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Text.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "string2_col".into(),
            tpe: ColumnType {
                full_data_type: "varchar".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::VarChar(Some(1)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigint_col".into(),
            tpe: ColumnType {
                full_data_type: "int8".into(),
                family: ColumnTypeFamily::BigInt,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::BigInt.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "bigserial_col".into(),
            tpe: ColumnType {
                full_data_type: "int8".into(),
                family: ColumnTypeFamily::BigInt,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::BigInt.to_json()),
            },
            default: Some(DefaultValue::sequence("User_bigserial_col_seq".to_string())),
            auto_increment: true,
        },
        Column {
            name: "bit_col".into(),
            tpe: ColumnType {
                full_data_type: "bit".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Bit(Some(1)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "bit_varying_col".into(),
            tpe: ColumnType {
                full_data_type: "varbit".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::VarBit(Some(1)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "box_col".into(),
            tpe: ColumnType {
                full_data_type: "box".into(),
                family: ColumnTypeFamily::Unsupported("box".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "char_col".into(),
            tpe: ColumnType {
                full_data_type: "bpchar".into(),
                family: ColumnTypeFamily::String,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Char(Some(1)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "circle_col".into(),
            tpe: ColumnType {
                full_data_type: "circle".into(),
                family: ColumnTypeFamily::Unsupported("circle".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "line_col".into(),
            tpe: ColumnType {
                full_data_type: "line".into(),
                family: ColumnTypeFamily::Unsupported("line".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "time_col".into(),
            tpe: ColumnType {
                full_data_type: "time".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Time(Some(6)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "timetz_col".into(),
            tpe: ColumnType {
                full_data_type: "timetz".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Timetz(Some(6)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamp_col".into(),
            tpe: ColumnType {
                full_data_type: "timestamp".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Timestamp(Some(6)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "timestamptz_col".into(),
            tpe: ColumnType {
                full_data_type: "timestamptz".into(),
                family: ColumnTypeFamily::DateTime,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Timestamptz(Some(6)).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "lseg_col".into(),
            tpe: ColumnType {
                full_data_type: "lseg".into(),
                family: ColumnTypeFamily::Unsupported("lseg".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "numeric_col".into(),
            tpe: ColumnType {
                full_data_type: "numeric".into(),
                family: ColumnTypeFamily::Decimal,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Decimal(None).to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "path_col".into(),
            tpe: ColumnType {
                full_data_type: "path".into(),
                family: ColumnTypeFamily::Unsupported("path".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "pg_lsn_col".into(),
            tpe: ColumnType {
                full_data_type: "pg_lsn".into(),
                family: ColumnTypeFamily::Unsupported("pg_lsn".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "polygon_col".into(),
            tpe: ColumnType {
                full_data_type: "polygon".into(),
                family: ColumnTypeFamily::Unsupported("polygon".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallint_col".into(),
            tpe: ColumnType {
                full_data_type: "int2".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::SmallInt.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "smallserial_col".into(),
            tpe: ColumnType {
                full_data_type: "int2".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::SmallInt.to_json()),
            },
            default: Some(DefaultValue::sequence("User_smallserial_col_seq".to_string())),
            auto_increment: true,
        },
        Column {
            name: "serial_col".into(),
            tpe: ColumnType {
                full_data_type: "int4".into(),
                family: ColumnTypeFamily::Int,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Integer.to_json()),
            },
            default: Some(DefaultValue::sequence("User_serial_col_seq".to_string())),
            auto_increment: true,
        },
        Column {
            name: "tsquery_col".into(),
            tpe: ColumnType {
                full_data_type: "tsquery".into(),
                family: ColumnTypeFamily::Unsupported("tsquery".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "tsvector_col".into(),
            tpe: ColumnType {
                full_data_type: "tsvector".into(),
                family: ColumnTypeFamily::Unsupported("tsvector".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "txid_col".into(),
            tpe: ColumnType {
                full_data_type: "txid_snapshot".into(),
                family: ColumnTypeFamily::Unsupported("txid_snapshot".into()),
                arity: ColumnArity::Required,
                native_type: None,
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "json_col".into(),
            tpe: ColumnType {
                full_data_type: "json".into(),
                family: ColumnTypeFamily::Json,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Json.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "jsonb_col".into(),
            tpe: ColumnType {
                full_data_type: "jsonb".into(),
                family: ColumnTypeFamily::Json,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::JsonB.to_json()),
            },
            default: None,
            auto_increment: false,
        },
        Column {
            name: "uuid_col".into(),
            tpe: ColumnType {
                full_data_type: "uuid".into(),
                family: ColumnTypeFamily::Uuid,
                arity: ColumnArity::Required,
                native_type: Some(PostgresType::Uuid.to_json()),
            },
            default: None,
            auto_increment: false,
        },
    ];
    expected_columns.sort_unstable_by_key(|c| c.name.to_owned());

    assert_eq!(
        table,
        &Table {
            name: "User".into(),
            columns: expected_columns,
            indices: vec![Index {
                name: "User_uuid_col_key".into(),
                columns: vec![IndexColumn {
                    name: "uuid_col".to_string(),
                    sort_order: Some(SQLSortOrder::Asc),
                    length: None,
                }],
                tpe: IndexType::Unique,
            },],
            primary_key: Some(PrimaryKey {
                columns: vec![PrimaryKeyColumn::new("primary_col")],
                constraint_name: Some("User_pkey".into()),
            }),
            foreign_keys: vec![],
        }
    );

    if api.connector_tags().contains(Tags::Postgres9) {
        return; // sequence max values work differently on postgres 9
    }

    let ext = extract_ext(&result);
    let expected_ext = expect![[r#"
        PostgresSchemaExt {
            opclasses: [],
            indexes: [
                (
                    IndexId(
                        TableId(
                            0,
                        ),
                        0,
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
fn postgres_cross_schema_references_are_not_allowed(api: TestApi) {
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
    let index = &table.indices[0];

    assert_eq!(
        &index.columns,
        &[
            IndexColumn {
                name: "name".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None,
            },
            IndexColumn {
                name: "age".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None,
            },
        ]
    );
    assert!(index.tpe.is_unique());

    let index = &table.indices[1];

    assert!(!index.tpe.is_unique());
    assert_eq!(
        &index.columns,
        &[
            IndexColumn {
                name: "age".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None,
            },
            IndexColumn {
                name: "name".to_string(),
                sort_order: Some(SQLSortOrder::Asc),
                length: None,
            },
        ]
    );
}

#[test_connector(tags(Postgres))]
fn escaped_quotes_in_string_defaults_must_be_unescaped(api: TestApi) {
    let create_table = format!(
        r#"
            CREATE TABLE "{0}"."string_defaults_test" (
                id INTEGER PRIMARY KEY,
                regular VARCHAR NOT NULL DEFAULT E'meow, says the cat',
                escaped VARCHAR NOT NULL DEFAULT E'"That\'s a lot of fish!" - Godzilla, 1998'
            );
        "#,
        api.schema_name()
    );

    api.raw_cmd(&create_table);

    let schema = api.describe();

    let table = schema.table_bang("string_defaults_test");

    let regular_column_default = table
        .column_bang("regular")
        .default
        .as_ref()
        .unwrap()
        .as_value()
        .unwrap()
        .clone()
        .into_string()
        .unwrap();

    assert_eq!(regular_column_default, "meow, says the cat");

    let escaped_column_default = table
        .column_bang("escaped")
        .default
        .as_ref()
        .unwrap()
        .as_value()
        .unwrap()
        .clone()
        .into_string()
        .unwrap();

    assert_eq!(escaped_column_default, r#""That's a lot of fish!" - Godzilla, 1998"#);
}

#[test_connector(tags(Postgres))]
fn escaped_backslashes_in_string_literals_must_be_unescaped(api: TestApi) {
    // https://www.postgresql.org/docs/current/sql-syntax-lexical.html
    let create_table = r#"
        CREATE TABLE test (
            "model_name_space" VARCHAR(255) NOT NULL DEFAULT e'xyz\\Datasource\\Model'
        )
    "#;

    api.raw_cmd(create_table);

    let schema = api.describe();

    let table = schema.table_bang("test");

    let default = table
        .column_bang("model_name_space")
        .default
        .as_ref()
        .unwrap()
        .as_value()
        .unwrap()
        .clone()
        .into_string()
        .unwrap();

    assert_eq!(default, r#"xyz\Datasource\Model"#);
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
    let index = table.index_at(0);

    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(1, columns.len());
    assert_eq!("a", columns[0].as_column().name());
    assert_eq!(Some(SQLSortOrder::Desc), columns[0].sort_order());
}

#[test_connector(tags(Postgres))]
fn index_sort_order_composite_type_desc_desc_is_handled(api: TestApi) {
    let sql = indoc! {r#"
        CREATE TABLE A (
            id INT PRIMARY KEY,
            a  INT NOT NULL,
            b  INT NOT NULL
        );

        CREATE INDEX foo ON A (a DESC, b DESC);
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let index = table.index_at(0);

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
            id INT PRIMARY KEY,
            a  INT NOT NULL,
            b  INT NOT NULL
        );

        CREATE INDEX foo ON A (a ASC, b DESC);
    "#};

    api.raw_cmd(sql);

    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();
    let index = table.index_at(0);

    let columns = index.columns().collect::<Vec<_>>();

    assert_eq!(2, columns.len());

    assert_eq!("a", columns[0].as_column().name());
    assert_eq!("b", columns[1].as_column().name());

    assert_eq!(Some(SQLSortOrder::Asc), columns[0].sort_order());
    assert_eq!(Some(SQLSortOrder::Desc), columns[1].sort_order());
}

#[test_connector(tags(Postgres))]
fn array_column_defaults(api: TestApi) {
    use prisma_value::PrismaValue;

    let schema = r#"
        CREATE TYPE "color" AS ENUM ('RED', 'GREEN', 'BLUE');

        CREATE TABLE "defaults" (
            text_empty TEXT[] NOT NULL DEFAULT '{}',
            text TEXT[] NOT NULL DEFAULT '{ ''abc'' }',
            colors COLOR[] NOT NULL DEFAULT '{ RED, GREEN }'
        );
    "#;

    api.raw_cmd(schema);
    let schema = api.describe();
    let table = schema.table_walkers().next().unwrap();

    let assert_default = |colname: &str, expected_default: Vec<PrismaValue>| {
        let col = table.column(colname).unwrap();
        let value = dbg!(col.default().unwrap()).as_value().unwrap();
        assert_eq!(value, &PrismaValue::List(expected_default));
    };

    assert_default("text_empty", vec![]);
    assert_default("text", vec!["abc".into()]);

    todo!();
}

fn extract_ext(schema: &SqlSchema) -> &PostgresSchemaExt {
    schema.downcast_connector_data().unwrap_or_default()
}

use super::utils::*;

use psl::{builtin_connectors::PostgresType, parser_database::ScalarType};
use quaint::prelude::ColumnType;
use sql_migration_tests::test_api::*;

#[test_connector(tags(Postgres, CockroachDb))]
fn enum_pg(api: TestApi) {
    api.schema_push(ENUM_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "int4",
                    typ: "int",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "MyFancyEnum",
                    typ: "MyFancyEnum",
                },
            ],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "id",
                    typ: "int",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "enum",
                    typ: "MyFancyEnum",
                },
            ],
        }
    "#]];

    api.introspect_sql(
        "test_1",
        "INSERT INTO model (id, enum) VALUES (?, ?) RETURNING id, enum;",
    )
    .send_sync()
    .expect_result(expected)
}

#[test_connector(tags(Postgres, CockroachDb))]
fn empty_result(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "int4",
                    typ: "int",
                },
            ],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "int",
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT int FROM model WHERE 1 = 0 AND int = ?;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Postgres, CockroachDb))]
fn unnamed_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "?column?",
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Postgres, CockroachDb))]
fn named_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "add",
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1 as add;")
        .send_sync()
        .expect_result(expected)
}

const DATASOURCE: &str = r#"
  datasource db {
    provider = "postgres"
    url      = "postgresql://localhost:5432"
}
"#;

macro_rules! test_scalar_types {
    ($
        ($test_name:ident($st:expr) => $ct:ident,)
    *) => {
        $(
            #[test_connector(tags(Postgres, CockroachDb))]
            fn $test_name(api: TestApi) {
                let dm = render_scalar_type_datamodel(DATASOURCE, $st);

                api.schema_push(&dm).send();

                let query = "INSERT INTO test (field) VALUES (?) RETURNING field;";

                api.introspect_sql("test", query)
                    .send_sync()
                    .expect_param_type(0, ColumnType::$ct)
                    .expect_column_type(0, ColumnType::$ct);
            }
        )*
    };
}

macro_rules! test_native_types {
    ($
        ($test_name:ident($nt:expr) => $ct:ident,)
    *) => {
        $(
            #[test_connector(tags(Postgres, CockroachDb))]
            fn $test_name(api: TestApi) {
                let dm = render_native_type_datamodel::<PostgresType>(&api, DATASOURCE, $nt.to_parts(), $nt);

                if $nt == PostgresType::Citext {
                    api.raw_cmd("CREATE EXTENSION IF NOT EXISTS citext;");
                }

                api.schema_push(&dm).send();

                let query = "INSERT INTO test (field) VALUES (?) RETURNING field;";

                api.introspect_sql("test", query)
                    .send_sync()
                    .expect_param_type(0, ColumnType::$ct)
                    .expect_column_type(0, ColumnType::$ct);
            }
        )*
    };
}

test_scalar_types! {
    int(ScalarType::Int) => Int32,
    string(ScalarType::String) => Text,
    bigint(ScalarType::BigInt) => Int64,
    float(ScalarType::Float) => Double,
    bytes(ScalarType::Bytes) => Bytes,
    bool(ScalarType::Boolean) => Boolean,
    datetime(ScalarType::DateTime) => DateTime,
    decimal(ScalarType::Decimal) => Numeric,
    json(ScalarType::Json) => Json,
}

test_native_types! {
    small_int(PostgresType::SmallInt) => Int32,
    integer(PostgresType::Integer) => Int32,
    big_int(PostgresType::BigInt) => Int64,
    nt_decimal(PostgresType::Decimal(Some((4, 4)))) => Numeric,
    money(PostgresType::Money) => Numeric,
    inet(PostgresType::Inet) => Text,
    oid(PostgresType::Oid) => Int64,
    citext(PostgresType::Citext) => Text,
    real(PostgresType::Real) => Float,
    double(PostgresType::DoublePrecision) => Double,
    var_char(PostgresType::VarChar(Some(255))) => Text,
    char(PostgresType::Char(Some(255))) => Text,
    text(PostgresType::Text) => Text,
    byte(PostgresType::ByteA) => Bytes,
    timestamp(PostgresType::Timestamp(Some(1))) => DateTime,
    timestamptz(PostgresType::Timestamptz(Some(1))) => DateTime,
    date(PostgresType::Date) => Date,
    time(PostgresType::Time(Some(1))) => Time,
    timetz(PostgresType::Timetz(Some(1))) => Time,
    boolean(PostgresType::Boolean) => Boolean,
    bit(PostgresType::Bit(Some(1))) => Text,
    var_bit(PostgresType::VarBit(Some(1))) => Text,
    uuid(PostgresType::Uuid) => Uuid,
    xml(PostgresType::Xml) => Xml,
    nt_json(PostgresType::Json) => Json,
    json_b(PostgresType::JsonB) => Json,
}

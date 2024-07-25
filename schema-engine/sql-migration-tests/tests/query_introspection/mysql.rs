use super::utils::*;
use psl::{builtin_connectors::MySqlType, parser_database::ScalarType};
use quaint::prelude::ColumnType;
use sql_migration_tests::test_api::*;

#[test_connector(tags(Mysql8))]
fn insert_mysql(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let query =
        "INSERT INTO `model` (`int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt`) VALUES (?, ?, ?, ?, ?, ?, ?);";

    let res = api.introspect_sql("test_1", query).send_sync();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_0",
                    typ: "bigint",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_1",
                    typ: "string",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_2",
                    typ: "bigint",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_3",
                    typ: "double",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_4",
                    typ: "bytes",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_5",
                    typ: "bigint",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_6",
                    typ: "datetime",
                },
            ],
            result_columns: [],
        }
    "#]];

    res.expect_result(expected);
}

#[test_connector(tags(Mysql, Mariadb))]
fn select_mysql(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let res = api
        .introspect_sql(
            "test_1",
            "SELECT `int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt` FROM `model`;",
        )
        .send_sync();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "int",
                    typ: "int",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "string",
                    typ: "string",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "bigint",
                    typ: "bigint",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "float",
                    typ: "double",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "bytes",
                    typ: "bytes",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "bool",
                    typ: "int",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "dt",
                    typ: "datetime",
                },
            ],
        }
    "#]];

    res.expect_result(expected);
}

#[test_connector(tags(Mysql8))]
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

#[test_connector(tags(Mysql8))]
fn unnamed_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "1 + 1",
                    typ: "bigint",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Mysql8))]
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
    provider = "mysql"
    url      = "mysql://localhost:5432"
}
"#;

macro_rules! test_native_types {
    (
        $tag:ident;

        $(
            $test_name:ident($nt:expr) => ($ct_input:ident, $ct_output:ident),
        )*
    ) => {
            $(
                paste::paste! {
                    #[test_connector]
                    fn [<nt _ $test_name _ $tag:lower>](api: TestApi) {
                        test_setup::only!($tag);

                        let dm = render_native_type_datamodel::<MySqlType>(&api, DATASOURCE, $nt.to_parts(), $nt);

                        api.schema_push(&dm).send();

                        api.introspect_sql("test_1", "INSERT INTO test (field) VALUES (?);")
                            .send_sync()
                            .expect_param_type(0, ColumnType::$ct_input);

                        api.introspect_sql("test_2", "SELECT field FROM test;")
                            .send_sync()
                            .expect_column_type(0, ColumnType::$ct_output);
                    }
                }
            )*
    };
}

test_scalar_types!(
    Mysql8;

    int(ScalarType::Int) => (Int64, Int32),
    string(ScalarType::String) => (Text, Text),
    bigint(ScalarType::BigInt) => (Int64, Int64),
    float(ScalarType::Float) => (Double, Double),
    bytes(ScalarType::Bytes) => (Bytes, Bytes),
    bool(ScalarType::Boolean) => (Int64, Int32),
    dt(ScalarType::DateTime) => (DateTime, DateTime),
    decimal(ScalarType::Decimal) => (Numeric, Numeric),
);

test_native_types! {
    Mysql8;

    int(MySqlType::Int) => (Int64, Int32),
    unsigned_int(MySqlType::UnsignedInt) => (Int64, Int64),
    small_int(MySqlType::SmallInt) => (Int64, Int32),
    unsigned_small_int(MySqlType::UnsignedSmallInt) => (Int64, Int32),
    tiny_int(MySqlType::TinyInt) => (Int64, Int32),
    unsigned_tiny_int(MySqlType::UnsignedTinyInt) => (Int64, Int32),
    medium_int(MySqlType::MediumInt) => (Int64, Int32),
    unsigned_medium_int(MySqlType::UnsignedMediumInt) => (Int64, Int64),
    big_int(MySqlType::BigInt) => (Int64, Int64),
    decimal(MySqlType::Decimal(Some((4, 4)))) => (Numeric, Numeric),
    unsigned_big_int(MySqlType::UnsignedBigInt) => (Int64, Int64),
    float(MySqlType::Float) => (Double, Float),
    double(MySqlType::Double) => (Double, Double),
    bit(MySqlType::Bit(1)) => (Bytes, Boolean),
    char(MySqlType::Char(255)) => (Text, Text),
    var_char(MySqlType::VarChar(255)) => (Text, Text),
    binary(MySqlType::Binary(255)) => (Bytes, Bytes),
    var_binary(MySqlType::VarBinary(255)) => (Bytes, Bytes),
    tiny_blob(MySqlType::TinyBlob) => (Bytes, Bytes),
    blob(MySqlType::Blob) => (Bytes, Bytes),
    medium_blob(MySqlType::MediumBlob) => (Bytes, Bytes),
    long_blob(MySqlType::LongBlob) => (Bytes, Bytes),
    tiny_text(MySqlType::TinyText) => (Text, Text),
    text(MySqlType::Text) => (Text, Text),
    medium_text(MySqlType::MediumText) => (Text, Text),
    long_text(MySqlType::LongText) => (Text, Text),
    date(MySqlType::Date) => (Date, Date),
    time(MySqlType::Time(Some(3))) => (Time, Time),
    date_time(MySqlType::DateTime(Some(3))) => (DateTime, DateTime),
    timestamp(MySqlType::Timestamp(Some(3))) => (DateTime, DateTime),
    year(MySqlType::Year) => (Int32, Int32),
    json(MySqlType::Json) => (Json, Json),
}

test_scalar_types!(
    Mysql57;

    int(ScalarType::Int) => (Bytes, Int32),
    string(ScalarType::String) => (Bytes, Text),
    bigint(ScalarType::BigInt) => (Bytes, Int64),
    float(ScalarType::Float) => (Bytes, Double),
    bytes(ScalarType::Bytes) => (Bytes, Bytes),
    bool(ScalarType::Boolean) => (Bytes, Int32),
    dt(ScalarType::DateTime) => (Bytes, DateTime),
    decimal(ScalarType::Decimal) => (Bytes, Numeric),
);

test_native_types! {
    Mysql57;

    int(MySqlType::Int) => (Bytes, Int32),
    unsigned_int(MySqlType::UnsignedInt) => (Bytes, Int64),
    small_int(MySqlType::SmallInt) => (Bytes, Int32),
    unsigned_small_int(MySqlType::UnsignedSmallInt) => (Bytes, Int32),
    tiny_int(MySqlType::TinyInt) => (Bytes, Int32),
    unsigned_tiny_int(MySqlType::UnsignedTinyInt) => (Bytes, Int32),
    medium_int(MySqlType::MediumInt) => (Bytes, Int32),
    unsigned_medium_int(MySqlType::UnsignedMediumInt) => (Bytes, Int64),
    big_int(MySqlType::BigInt) => (Bytes, Int64),
    decimal(MySqlType::Decimal(Some((4, 4)))) => (Bytes, Numeric),
    unsigned_big_int(MySqlType::UnsignedBigInt) => (Bytes, Int64),
    float(MySqlType::Float) => (Bytes, Float),
    double(MySqlType::Double) => (Bytes, Double),
    bit(MySqlType::Bit(1)) => (Bytes, Boolean),
    char(MySqlType::Char(255)) => (Bytes, Text),
    var_char(MySqlType::VarChar(255)) => (Bytes, Text),
    binary(MySqlType::Binary(255)) => (Bytes, Bytes),
    var_binary(MySqlType::VarBinary(255)) => (Bytes, Bytes),
    tiny_blob(MySqlType::TinyBlob) => (Bytes, Bytes),
    blob(MySqlType::Blob) => (Bytes, Bytes),
    medium_blob(MySqlType::MediumBlob) => (Bytes, Bytes),
    long_blob(MySqlType::LongBlob) => (Bytes, Bytes),
    tiny_text(MySqlType::TinyText) => (Bytes, Text),
    text(MySqlType::Text) => (Bytes, Text),
    medium_text(MySqlType::MediumText) => (Bytes, Text),
    long_text(MySqlType::LongText) => (Bytes, Text),
    date(MySqlType::Date) => (Bytes, Date),
    time(MySqlType::Time(Some(3))) => (Bytes, Time),
    date_time(MySqlType::DateTime(Some(3))) => (Bytes, DateTime),
    timestamp(MySqlType::Timestamp(Some(3))) => (Bytes, DateTime),
    year(MySqlType::Year) => (Bytes, Int32),
    json(MySqlType::Json) => (Bytes, Json),
}

test_scalar_types!(
    Mysql56;

    int(ScalarType::Int) => (Bytes, Int32),
    string(ScalarType::String) => (Bytes, Text),
    bigint(ScalarType::BigInt) => (Bytes, Int64),
    float(ScalarType::Float) => (Bytes, Double),
    bytes(ScalarType::Bytes) => (Bytes, Bytes),
    bool(ScalarType::Boolean) => (Bytes, Int32),
    dt(ScalarType::DateTime) => (Bytes, DateTime),
    decimal(ScalarType::Decimal) => (Bytes, Numeric),
);

test_native_types! {
    Mysql56;

    int(MySqlType::Int) => (Bytes, Int32),
    unsigned_int(MySqlType::UnsignedInt) => (Bytes, Int64),
    small_int(MySqlType::SmallInt) => (Bytes, Int32),
    unsigned_small_int(MySqlType::UnsignedSmallInt) => (Bytes, Int32),
    tiny_int(MySqlType::TinyInt) => (Bytes, Int32),
    unsigned_tiny_int(MySqlType::UnsignedTinyInt) => (Bytes, Int32),
    medium_int(MySqlType::MediumInt) => (Bytes, Int32),
    unsigned_medium_int(MySqlType::UnsignedMediumInt) => (Bytes, Int64),
    big_int(MySqlType::BigInt) => (Bytes, Int64),
    decimal(MySqlType::Decimal(Some((4, 4)))) => (Bytes, Numeric),
    unsigned_big_int(MySqlType::UnsignedBigInt) => (Bytes, Int64),
    float(MySqlType::Float) => (Bytes, Float),
    double(MySqlType::Double) => (Bytes, Double),
    bit(MySqlType::Bit(1)) => (Bytes, Boolean),
    char(MySqlType::Char(255)) => (Bytes, Text),
    var_char(MySqlType::VarChar(255)) => (Bytes, Text),
    binary(MySqlType::Binary(255)) => (Bytes, Bytes),
    var_binary(MySqlType::VarBinary(255)) => (Bytes, Bytes),
    tiny_blob(MySqlType::TinyBlob) => (Bytes, Bytes),
    blob(MySqlType::Blob) => (Bytes, Bytes),
    medium_blob(MySqlType::MediumBlob) => (Bytes, Bytes),
    long_blob(MySqlType::LongBlob) => (Bytes, Bytes),
    tiny_text(MySqlType::TinyText) => (Bytes, Text),
    text(MySqlType::Text) => (Bytes, Text),
    medium_text(MySqlType::MediumText) => (Bytes, Text),
    long_text(MySqlType::LongText) => (Bytes, Text),
    date(MySqlType::Date) => (Bytes, Date),
    time(MySqlType::Time(Some(3))) => (Bytes, Time),
    date_time(MySqlType::DateTime(Some(3))) => (Bytes, DateTime),
    timestamp(MySqlType::Timestamp(Some(3))) => (Bytes, DateTime),
    year(MySqlType::Year) => (Bytes, Int32),
    json(MySqlType::Json) => (Bytes, Json),
}

test_scalar_types!(
    Mariadb;

    int(ScalarType::Int) => (Unknown, Int32),
    string(ScalarType::String) => (Unknown, Text),
    bigint(ScalarType::BigInt) => (Unknown, Int64),
    float(ScalarType::Float) => (Unknown, Double),
    bytes(ScalarType::Bytes) => (Unknown, Bytes),
    bool(ScalarType::Boolean) => (Unknown, Int32),
    dt(ScalarType::DateTime) => (Unknown, DateTime),
    decimal(ScalarType::Decimal) => (Unknown, Numeric),
);

test_native_types! {
    Mariadb;

    int(MySqlType::Int) => (Unknown, Int32),
    unsigned_int(MySqlType::UnsignedInt) => (Unknown, Int64),
    small_int(MySqlType::SmallInt) => (Unknown, Int32),
    unsigned_small_int(MySqlType::UnsignedSmallInt) => (Unknown, Int32),
    tiny_int(MySqlType::TinyInt) => (Unknown, Int32),
    unsigned_tiny_int(MySqlType::UnsignedTinyInt) => (Unknown, Int32),
    medium_int(MySqlType::MediumInt) => (Unknown, Int32),
    unsigned_medium_int(MySqlType::UnsignedMediumInt) => (Unknown, Int64),
    big_int(MySqlType::BigInt) => (Unknown, Int64),
    decimal(MySqlType::Decimal(Some((4, 4)))) => (Unknown, Numeric),
    unsigned_big_int(MySqlType::UnsignedBigInt) => (Unknown, Int64),
    float(MySqlType::Float) => (Unknown, Float),
    double(MySqlType::Double) => (Unknown, Double),
    bit(MySqlType::Bit(1)) => (Unknown, Boolean),
    char(MySqlType::Char(255)) => (Unknown, Text),
    var_char(MySqlType::VarChar(255)) => (Unknown, Text),
    binary(MySqlType::Binary(255)) => (Unknown, Bytes),
    var_binary(MySqlType::VarBinary(255)) => (Unknown, Bytes),
    tiny_blob(MySqlType::TinyBlob) => (Unknown, Bytes),
    blob(MySqlType::Blob) => (Unknown, Bytes),
    medium_blob(MySqlType::MediumBlob) => (Unknown, Bytes),
    long_blob(MySqlType::LongBlob) => (Unknown, Bytes),
    tiny_text(MySqlType::TinyText) => (Unknown, Text),
    text(MySqlType::Text) => (Unknown, Text),
    medium_text(MySqlType::MediumText) => (Unknown, Text),
    long_text(MySqlType::LongText) => (Unknown, Text),
    date(MySqlType::Date) => (Unknown, Date),
    time(MySqlType::Time(Some(3))) => (Unknown, Time),
    date_time(MySqlType::DateTime(Some(3))) => (Unknown, DateTime),
    timestamp(MySqlType::Timestamp(Some(3))) => (Unknown, DateTime),
    year(MySqlType::Year) => (Unknown, Int32),
    json(MySqlType::Json) => (Unknown, Text),
}

use quaint::Value;
use sql_migration_tests::test_api::*;

const SIMPLE_SCHEMA: &str = r#"
model model {
    int     Int     @id
    string  String
    bigint  BigInt
    float   Float
    bytes   Bytes
    bool    Boolean
    dt      DateTime
}"#;

const ENUM_SCHEMA: &str = r#"
model model {
    id     Int     @id
    enum    MyFancyEnum
}

enum MyFancyEnum {
    A
    B
    C
}
"#;

fn typ_to_value(typ: &str) -> Value<'static> {
    match typ {
        "string" => Value::text("hello"),
        "int" => Value::int32(i8::MAX),
        "bigint" => Value::int64(i8::MAX),
        "float" => Value::float(f32::EPSILON),
        "double" => Value::double(f64::EPSILON),
        "bytes" => Value::bytes("hello".as_bytes()),
        "bool" => Value::boolean(false),
        "datetime" => Value::datetime(
            chrono::DateTime::parse_from_rfc3339("2020-02-27T19:10:00Z")
                .unwrap()
                .into(),
        ),
        _ => unimplemented!(),
    }
}

#[test_connector(tags(Postgres, CockroachDb))]
fn insert_pg(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let query = "INSERT INTO model (int, string, bigint, float, bytes, bool, dt) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING int, string, bigint, float, bytes, bool, dt;";
    let res = api.introspect_sql("test_1", query).send_sync();

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
                    name: "text",
                    typ: "string",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "int8",
                    typ: "bigint",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "float8",
                    typ: "double",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "bytea",
                    typ: "bytes",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "bool",
                    typ: "bool",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "timestamp",
                    typ: "datetime",
                },
            ],
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
                    typ: "bool",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "dt",
                    typ: "datetime",
                },
            ],
        }
    "#]];

    res.expect_result(expected);

    let values = res
        .output
        .parameters
        .iter()
        .map(|param| typ_to_value(&param.typ))
        .collect::<Vec<_>>();

    api.query_raw(&api.sanitize_sql(&query), &values);
}

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

#[test_connector(tags(Mysql, Mariadb))]
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

    let values = res
        .output
        .parameters
        .iter()
        .map(|param| typ_to_value(&param.typ))
        .collect::<Vec<_>>();

    api.query_raw(&query, &values);
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

#[test_connector(tags(Sqlite))]
fn insert_sqlite(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let query =
        "INSERT INTO `model` (`int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt`) VALUES (?, ?, ?, ?, ?, ?, ?);";

    let res = api.introspect_sql("test_1", query).send_sync();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "_1",
                    typ: "unknown",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "_2",
                    typ: "unknown",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "_3",
                    typ: "unknown",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "_4",
                    typ: "unknown",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "_5",
                    typ: "unknown",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "_6",
                    typ: "unknown",
                },
            ],
        }
    "#]];

    res.expect_result(expected);
}

#[test_connector(tags(Sqlite))]
fn select_sqlite(api: TestApi) {
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
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_0",
                    typ: "int",
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
                    typ: "bool",
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

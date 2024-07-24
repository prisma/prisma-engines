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

#[test_connector(tags(Postgres, CockroachDb))]
fn insert_pg(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            documentation: "",
            name: "test_1",
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$0",
                    typ: "int",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$1",
                    typ: "string",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$2",
                    typ: "bigint",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$3",
                    typ: "double",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$4",
                    typ: "bytes",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$5",
                    typ: "bool",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "$6",
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

    api.introspect_sql("test_1", "INSERT INTO model (int, string, bigint, float, bytes, bool, dt) VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING int, string, bigint, float, bytes, bool, dt;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Mysql, Mariadb))]
fn insert_mysql(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

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

    api.introspect_sql(
        "test_1",
        "INSERT INTO `model` (`int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt`) VALUES (?, ?, ?, ?, ?, ?, ?);",
    )
    .send_sync()
    .expect_result(expected)
}

#[test_connector(tags(Mysql, Mariadb))]
fn select_mysql(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

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

    api.introspect_sql(
        "test_1",
        "SELECT `int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt` FROM `model`;",
    )
    .send_sync()
    .expect_result(expected)
}

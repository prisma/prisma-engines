use super::utils::*;

use sql_migration_tests::test_api::*;

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
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_1",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_2",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_3",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_4",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_5",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_6",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: "",
                    name: "_7",
                    typ: "unknown",
                },
            ],
            result_columns: [],
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
}

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

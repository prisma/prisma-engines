use super::utils::*;

use sql_migration_tests::test_api::*;

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

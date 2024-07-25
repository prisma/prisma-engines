use super::utils::*;

use sql_migration_tests::test_api::*;

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

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
            name: "test_1",
            source: "INSERT INTO `model` (`int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt`) VALUES (?, ?, ?, ?, ?, ?, ?);",
            documentation: None,
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_1",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_2",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_3",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_4",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_5",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_6",
                    typ: "unknown",
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
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
            name: "test_1",
            source: "SELECT `int`, `string`, `bigint`, `float`, `bytes`, `bool`, `dt` FROM `model`;",
            documentation: None,
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

#[test_connector(tags(Sqlite))]
fn empty_result(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT int FROM model WHERE 1 = 0 AND int = ?;",
            documentation: None,
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_1",
                    typ: "unknown",
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

#[test_connector(tags(Sqlite))]
fn unnamed_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT 1 + 1;",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "1 + 1",
                    typ: "unknown",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn named_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT 1 + 1 as \"add\";",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "add",
                    typ: "unknown",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1 as \"add\";")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn mixed_named_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT `int` + 1 as \"add\" FROM `model`;",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "add",
                    typ: "unknown",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT `int` + 1 as \"add\" FROM `model`;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn mixed_unnamed_expr(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT `int` + 1 FROM `model`;",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "`int` + 1",
                    typ: "unknown",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT `int` + 1 FROM `model`;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn mixed_expr_cast(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT CAST(`int` + 1 as int) FROM `model`;",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "CAST(`int` + 1 as int)",
                    typ: "unknown",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT CAST(`int` + 1 as int) FROM `model`;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn subquery(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT `int` FROM (SELECT * FROM `model`)",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "int",
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT `int` FROM (SELECT * FROM `model`)")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn left_join(api: TestApi) {
    api.schema_push(RELATION_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT `parent`.`id` as `parentId`, `child`.`id` as `childId` FROM `parent` LEFT JOIN `child` ON `parent`.`id` = `child`.`parent_id`",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "parentId",
                    typ: "int",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "childId",
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT `parent`.`id` as `parentId`, `child`.`id` as `childId` FROM `parent` LEFT JOIN `child` ON `parent`.`id` = `child`.`parent_id`")
        .send_sync()
        .expect_result(expected)
}

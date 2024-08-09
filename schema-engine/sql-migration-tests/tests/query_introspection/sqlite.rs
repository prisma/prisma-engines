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
fn unnamed_expr_int(api: TestApi) {
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
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn named_expr_int(api: TestApi) {
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
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1 + 1 as \"add\";")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn mixed_named_expr_int(api: TestApi) {
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
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT `int` + 1 as \"add\" FROM `model`;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn mixed_unnamed_expr_int(api: TestApi) {
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
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT `int` + 1 FROM `model`;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn mixed_expr_cast_int(api: TestApi) {
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
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT CAST(`int` + 1 as int) FROM `model`;")
        .send_sync()
        .expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_string(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT 'hello world';",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "'hello world'",
                    typ: "string",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 'hello world';")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT 'hello world' as `str`;", &[])
        .assert_single_row(|row| row.assert_text_value("str", "hello world"));
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_bool(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT 1=1, 1=0;",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "1=1",
                    typ: "int",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "1=0",
                    typ: "int",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1=1, 1=0;")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT 1=1 as `true`, 1=0 AS `false`;", &[])
        .assert_single_row(|row| row.assert_int_value("true", 1).assert_int_value("false", 0));
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_real(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT 1.2, 2.34567891023, round(2.345);",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "1.2",
                    typ: "double",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "2.34567891023",
                    typ: "double",
                },
                IntrospectSqlQueryColumnOutput {
                    name: "round(2.345)",
                    typ: "double",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT 1.2, 2.34567891023, round(2.345);")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT 1.2 AS a, 2.34567891023 AS b, round(2.345) AS c;", &[])
        .assert_single_row(|row| {
            row.assert_float_value("a", 1.2)
                .assert_float_value("b", 2.34567891023)
                .assert_float_value("c", 2.0)
        });
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_blob(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT unhex('537475666673');",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "unhex('537475666673')",
                    typ: "bytes",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT unhex('537475666673');")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT unhex('537475666673') as blob;", &[])
        .assert_single_row(|row| row.assert_bytes_value("blob", &[83, 116, 117, 102, 102, 115]));
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_date(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT date('2025-05-29 14:16:00');",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "date('2025-05-29 14:16:00')",
                    typ: "string",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT date('2025-05-29 14:16:00');")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT date('2025-05-29 14:16:00') as dt;", &[])
        .assert_single_row(|row| row.assert_text_value("dt", "2025-05-29"));
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_time(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT time('2025-05-29 14:16:00');",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "time('2025-05-29 14:16:00')",
                    typ: "string",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT time('2025-05-29 14:16:00');")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT time('2025-05-29 14:16:00') as dt;", &[])
        .assert_single_row(|row| row.assert_text_value("dt", "14:16:00"));
}

#[test_connector(tags(Sqlite))]
fn unnamed_expr_datetime(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "SELECT datetime('2025-05-29 14:16:00');",
            documentation: None,
            parameters: [],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "datetime('2025-05-29 14:16:00')",
                    typ: "string",
                },
            ],
        }
    "#]];

    api.introspect_sql("test_1", "SELECT datetime('2025-05-29 14:16:00');")
        .send_sync()
        .expect_result(expected);

    api.query_raw("SELECT datetime('2025-05-29 14:16:00') as dt;", &[])
        .assert_single_row(|row| row.assert_text_value("dt", "2025-05-29 14:16:00"));
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

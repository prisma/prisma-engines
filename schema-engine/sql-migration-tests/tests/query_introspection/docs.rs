use super::utils::*;
use sql_migration_tests::test_api::*;

#[test_connector(tags(Postgres))]
fn parses_doc_complex(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "\n       --    @description   some  fancy   query\n  -- @param  {Int}   $1:myInt some integer\n      --   @param   {String?}$2:myString    some   string\n        -- @param {?} $3\n    SELECT int FROM model WHERE int = $1 and string = $2 and float = $3;\n    ",
            documentation: Some(
                "some  fancy   query",
            ),
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: Some(
                        "some integer",
                    ),
                    name: "myInt",
                    typ: "Int",
                    nullable: false,
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: Some(
                        "some   string",
                    ),
                    name: "myString",
                    typ: "String",
                    nullable: true,
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "float8",
                    typ: "double",
                    nullable: true,
                },
            ],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "int",
                    typ: "int",
                    nullable: false,
                },
            ],
        }
    "#]];

    let sql = r#"
       --    @description   some  fancy   query
  -- @param  {Int}   $1:myInt some integer
      --   @param   {String?}$2:myString    some   string
        -- @param {?} $3
    SELECT int FROM model WHERE int = ? and string = ? and float = ?;
    "#;

    api.introspect_sql("test_1", sql).send_sync().expect_result(expected)
}

#[test_connector(tags(Sqlite))]
fn parses_doc_no_position(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "\n  -- @param  {String} :myInt some integer\n    SELECT int FROM model WHERE int = :myInt and string = ?;\n    ",
            documentation: None,
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: Some(
                        "some integer",
                    ),
                    name: "myInt",
                    typ: "String",
                    nullable: false,
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "_2",
                    typ: "unknown",
                    nullable: false,
                },
            ],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "int",
                    typ: "int",
                    nullable: false,
                },
            ],
        }
    "#]];

    let sql = r#"
  -- @param  {String} :myInt some integer
    SELECT int FROM model WHERE int = :myInt and string = ?;
    "#;

    api.introspect_sql("test_1", sql).send_sync().expect_result(expected)
}

#[test_connector(tags(Postgres))]
fn parses_doc_no_alias(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let expected = expect![[r#"
        IntrospectSqlQueryOutput {
            name: "test_1",
            source: "\n  -- @param  {String} $2 some string\n    SELECT int FROM model WHERE int = $1 and string = $2;\n    ",
            documentation: None,
            parameters: [
                IntrospectSqlQueryParameterOutput {
                    documentation: None,
                    name: "int4",
                    typ: "int",
                    nullable: false,
                },
                IntrospectSqlQueryParameterOutput {
                    documentation: Some(
                        "some string",
                    ),
                    name: "text",
                    typ: "String",
                    nullable: false,
                },
            ],
            result_columns: [
                IntrospectSqlQueryColumnOutput {
                    name: "int",
                    typ: "int",
                    nullable: false,
                },
            ],
        }
    "#]];

    let sql = r#"
  -- @param  {String} $2 some string
    SELECT int FROM model WHERE int = $1 and string = $2;
    "#;

    api.introspect_sql("test_1", sql).send_sync().expect_result(expected)
}

#[test_connector(tags(Postgres))]
fn invalid_position_fails(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let sql = r#"
  -- @param  {Int} $hello:myInt some integer
    SELECT int FROM model WHERE int = ? and string = ?;
    "#;

    let expected = expect![
        "SQL documentation parsing: invalid position. Expected a number found: hello at ' $hello:myInt some integer'."
    ];

    expected.assert_eq(
        api.introspect_sql("test_1", sql)
            .send_unwrap_err()
            .message()
            .unwrap_or_default(),
    );
}

#[test_connector(tags(Postgres))]
fn unknown_type_fails(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let sql = r#"
  -- @param  {Hello} $hello:myInt some integer
    SELECT int FROM model WHERE int = ? and string = ?;
    "#;

    let expected = expect!["SQL documentation parsing: invalid type: 'Hello' (accepted types are: 'Int', 'BigInt', 'Float', 'Boolean', 'String', 'DateTime', 'Json', 'Bytes', 'Decimal') at '{Hello} $hello:myInt some integer'."];

    expected.assert_eq(
        api.introspect_sql("test_1", sql)
            .send_unwrap_err()
            .message()
            .unwrap_or_default(),
    );
}

#[test_connector(tags(Postgres))]
fn duplicate_param_position_fails(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let sql = r#"
  -- @param  {Int} $1:myInt
  -- @param  {String} $1:myString
    SELECT int FROM model WHERE int = ? and string = ?;
    "#;

    let expected = expect!["SQL documentation parsing: duplicate parameter (position or alias is already used) at '@param  {String} $1:myString'."];

    expected.assert_eq(
        api.introspect_sql("test_1", sql)
            .send_unwrap_err()
            .message()
            .unwrap_or_default(),
    );
}

#[test_connector(tags(Postgres))]
fn duplicate_param_name_fails(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let sql = r#"
  -- @param  {Int} $1:myInt
  -- @param  {String} $2:myInt
    SELECT int FROM model WHERE int = ? and string = ?;
    "#;

    let expected = expect!["SQL documentation parsing: duplicate parameter (position or alias is already used) at '@param  {String} $2:myInt'."];

    expected.assert_eq(
        api.introspect_sql("test_1", sql)
            .send_unwrap_err()
            .message()
            .unwrap_or_default(),
    );
}

#[test_connector(tags(Postgres))]
fn missing_param_position_or_alias_fails(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let sql = r#"
  -- @param  {Int} myInt
    SELECT int FROM model WHERE int = ? and string = ?;
    "#;

    let expected =
        expect!["SQL documentation parsing: missing position or alias (eg: $1:alias) at '@param  {Int} myInt'."];

    expected.assert_eq(
        api.introspect_sql("test_1", sql)
            .send_unwrap_err()
            .message()
            .unwrap_or_default(),
    );
}

#[test_connector(tags(Postgres))]
fn missing_everything_fails(api: TestApi) {
    api.schema_push(SIMPLE_SCHEMA).send().assert_green();

    let sql = r#"
  -- @param
    SELECT int FROM model WHERE int = ? and string = ?;
    "#;

    let expected =
        expect!["SQL documentation parsing: invalid parameter: could not parse any information at '@param'."];

    expected.assert_eq(
        api.introspect_sql("test_1", sql)
            .send_unwrap_err()
            .message()
            .unwrap_or_default(),
    );
}

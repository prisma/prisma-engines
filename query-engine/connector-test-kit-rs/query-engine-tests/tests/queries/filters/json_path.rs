use query_engine_tests::*;

#[test_suite(schema(schemas::json), only(Postgres, MySql(5.7, 8, "mariadb")))]
mod json_path {
    use query_engine_tests::{assert_error, is_one_of, run_query, ConnectorTag};

    #[connector_test]
    async fn no_path_without_filter(runner: Runner) -> TestResult<()> {
        assert_error!(
            runner,
            jsonq(&runner, json_path(&runner), Some("")),
            2019,
            "A JSON path cannot be set without a scalar filter."
        );

        Ok(())
    }

    #[connector_test(capabilities(JsonFilteringArrayPath))]
    async fn extract_array_path(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"{ \"a\": { \"b\": \"c\" } }"#, false).await?;
        create_row(&runner, 2, r#"{ \"a\": { \"b\": [1, 2, 3] } }"#, false).await?;
        create_row(&runner, 3, r#"{ \"a\": { \"b\": null } }"#, false).await?;
        create_row(&runner, 4, r#"{ \"a\": { \"b\": [null] } }"#, false).await?;
        create_row(&runner, 5, r#"{ }"#, false).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b"], equals: "\"c\"" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b", "0"], equals: "1" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b", "0"], equals: JsonNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b"], equals: JsonNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b"], equals: DbNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":5}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b"], equals: AnyNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":5}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(JsonFilteringJsonPath), only(MySql(5.7), MySql(8)))]
    async fn extract_json_path(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"{ \"a\": { \"b\": \"c\" } }"#, false).await?;
        create_row(&runner, 2, r#"{ \"a\": { \"b\": [1, 2, 3] } }"#, false).await?;
        create_row(&runner, 3, r#"{ \"a\": { \"b\": 1 } }"#, false).await?;
        create_row(&runner, 4, r#"{ \"a\": { \"b\": null } }"#, false).await?;
        create_row(&runner, 5, r#"{ \"a\": { \"b\": [null] } }"#, false).await?;
        create_row(&runner, 6, r#"{ }"#, false).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b", equals: "\"c\"" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b[0]", equals: "1" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b[0]", equals: JsonNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b", equals: DbNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":6}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b", equals: AnyNull "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":6}]}}"###
        );

        Ok(())
    }

    // TODO: MariaDB is excluded because it produces different results than MySQL and Postgres
    #[connector_test(only(Postgres, MySql(5.7, 8)))]
    async fn array_contains(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"3"#, true).await?;
        create_row(&runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 5, r#"\"a\""#, true).await?;
        create_row(&runner, 6, r#"[[1, 2]]"#, true).await?;
        create_row(&runner, 7, r#"[1, null, 2]"#, true).await?;
        create_row(&runner, 8, r#"[1, [null], 2]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_contains: "[3]""#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_contains: "[\"a\"]""#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_contains: "[[1, 2]]""#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":6}]}}"###
        );

        // MySQL has slightly different semantics and also coerces null to [null].
        is_one_of!(
            run_query!(runner, jsonq(&runner, r#"array_contains: "null""#, None)),
            vec![
                r#"{"data":{"findManyTestModel":[{"id":7}]}}"#,
                r#"{"data":{"findManyTestModel":[{"id":7},{"id":8}]}}"#
            ]
        );

        is_one_of!(
            run_query!(runner, jsonq(&runner, r#"array_contains: "[null]""#, None)),
            vec![
                r#"{"data":{"findManyTestModel":[{"id":7}]}}"#,
                r#"{"data":{"findManyTestModel":[{"id":7},{"id":8}]}}"#
            ]
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_contains: "[[null]]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":8}]}}"###
        );

        Ok(())
    }

    // TODO: MariaDB is excluded because it doesn't support array_starts_with yet
    #[connector_test(only(Postgres, MySql(5.7, 8)))]
    async fn array_starts_with(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"3"#, true).await?;
        create_row(&runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 5, r#"\"a\""#, true).await?;
        create_row(&runner, 6, r#"[[1, 2]]"#, true).await?;
        create_row(&runner, 7, r#"null"#, true).await?;
        create_row(&runner, 8, r#"[null, \"test\"]"#, true).await?;
        create_row(&runner, 9, r#"[[null], \"test\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "\"a\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "[1, 2]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":6}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "null" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":8}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "[null]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":9}]}}"###
        );

        Ok(())
    }

    // TODO: MariaDB is excluded because array_ends_with doesn't work yet
    #[connector_test(only(Postgres, MySql(5.7, 8)))]
    async fn array_ends_with(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 4, r#"[[1, 2], [3, 4]]"#, true).await?;
        create_row(&runner, 7, r#"null"#, true).await?;
        create_row(&runner, 8, r#"[\"test\", null]"#, true).await?;
        create_row(&runner, 9, r#"[\"test\", [null]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":3}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "[3, 4]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "null" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":8}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "[null]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":9}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn string_contains(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"fool\""#, true).await?;
        create_row(&runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_contains: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        Ok(())
    }

    #[connector_test]
    async fn string_starts_with(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"fool\""#, true).await?;
        create_row(&runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_starts_with: "foo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );
        Ok(())
    }

    #[connector_test]
    async fn string_ends_with(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"fool\""#, true).await?;
        create_row(&runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_ends_with: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );
        Ok(())
    }

    #[connector_test]
    async fn gt_gte(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"bar\""#, true).await?;
        create_row(&runner, 3, r#"1"#, true).await?;
        create_row(&runner, 4, r#"2"#, true).await?;
        create_row(&runner, 5, r#"1.4"#, true).await?;
        create_row(&runner, 6, r#"100"#, true).await?;
        create_row(&runner, 7, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gt: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gte: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gt: "1" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":5},{"id":6}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gte: "1" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn lt_lte(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"bar\""#, true).await?;
        create_row(&runner, 3, r#"1"#, true).await?;
        create_row(&runner, 4, r#"2"#, true).await?;
        create_row(&runner, 5, r#"1.4"#, true).await?;
        create_row(&runner, 6, r#"100"#, true).await?;
        create_row(&runner, 7, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lt: "\"f\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lte: "\"foo\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lt: "100" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lte: "100" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":3},{"id":4},{"id":5},{"id":6}]}}"###
        );

        Ok(())
    }

    // TODO: MariaDB is excluded because array_starts_with doesn't work yet
    #[connector_test(only(Postgres, MySql(5.7, 8)))]
    async fn multi_filtering(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"[3, 4, 6]"#, true).await?;
        create_row(&runner, 4, r#"[5, 6, 7]"#, true).await?;
        create_row(&runner, 5, r#"1"#, true).await?;
        create_row(&runner, 6, r#"2.4"#, true).await?;
        create_row(&runner, 7, r#"3"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                format!(r#"query {{
                    findManyTestModel(
                        where: {{ json: {{ {}, array_contains: "3", array_starts_with: "3" }} }},
                        cursor: {{ id: 2 }},
                        take: 2
                    ) {{ json }}
                }}"#, json_path(&runner))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[3,4,5]}}"},{"json":"{\"a\":{\"b\":[3,4,6]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                format!(r#"query {{
                    findManyTestModel(
                        where: {{
                            AND: [
                                {{ json: {{ {}, gte: "1" }} }},
                                {{ json: {{ {}, lt: "3" }} }},
                            ]
                        }}
                    ) {{ json }}
                }}"#, json_path(&runner), json_path(&runner))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":1}}"},{"json":"{\"a\":{\"b\":2.4}}"}]}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, id: u32, data: &str, nested: bool) -> TestResult<()> {
        let json = if nested {
            format!(r#"{{ \"a\": {{ \"b\": {} }} }}"#, data)
        } else {
            data.to_owned()
        };
        let q = format!(
            r#"mutation {{ createOneTestModel(data: {{ id: {}, json: "{}" }}) {{ id }} }}"#,
            id, json
        );

        runner.query(q).await?.assert_success();
        Ok(())
    }

    fn jsonq(runner: &Runner, filter: &str, path: Option<&str>) -> String {
        let path = path.unwrap_or_else(|| json_path(runner));

        format!(
            r#"query {{ findManyTestModel(where: {{ json: {{ {}, {} }} }} ) {{ id }} }}"#,
            filter, path
        )
    }

    fn json_path(runner: &Runner) -> &'static str {
        match runner.connector() {
            ConnectorTag::Postgres(_) => r#"path: ["a", "b"]"#,
            ConnectorTag::MySql(_) => r#"path: "$.a.b""#,
            x => unreachable!("JSON filtering is not supported on {}", x),
        }
    }
}

use query_engine_tests::*;

#[test_suite(schema(schemas::json), only(Postgres))]
mod json_path {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query, ConnectorTag, MySqlVersion, Runner};

    fn pg_json() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                json Json @test.Json
            }"#
        };

        schema.to_owned()
    }

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

    async fn extract_array_path_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"{ \"a\": { \"b\": \"c\" } }"#, false).await?;
        create_row(&runner, 2, r#"{ \"a\": { \"b\": [1, 2, 3] } }"#, false).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b"], equals: "\"c\"" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"c\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: ["a", "b", "0"], equals: "1" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(pg_json), capabilities(JsonFilteringArrayPath))]
    async fn extract_array_path_pg_json(runner: Runner) -> TestResult<()> {
        extract_array_path_runner(runner).await?;

        Ok(())
    }

    #[connector_test(capabilities(JsonFilteringArrayPath))]
    async fn extract_array_path(runner: Runner) -> TestResult<()> {
        extract_array_path_runner(runner).await?;

        Ok(())
    }

    #[connector_test(capabilities(JsonFilteringJsonPath), only(MySql(5.7), MySql(8)))]
    async fn extract_json_path(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"{ \"a\": { \"b\": \"c\" } }"#, false).await?;
        create_row(&runner, 2, r#"{ \"a\": { \"b\": [1, 2, 3] } }"#, false).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b", equals: "\"c\"" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"c\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"path: "$.a.b[0]", equals: "1" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"}]}}"###
        );

        Ok(())
    }

    async fn array_contains_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"3"#, true).await?;
        create_row(&runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 5, r#"\"a\""#, true).await?;
        create_row(&runner, 6, r#"[[1, 2]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_contains: "[3]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"},{"json":"{\"a\":{\"b\":[3,4,5]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_contains: "[\"a\"]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[\"a\",\"b\"]}}"}]}}"###
        );

        match runner.connector() {
            // MariaDB does not support finding arrays in arrays, unlike MySQL
            ConnectorTag::MySql(mysql) if mysql.version() == Some(&MySqlVersion::MariaDb) => {
                insta::assert_snapshot!(
                    run_query!(
                        runner,
                        jsonq(&runner, r#"array_contains: "[[1, 2]]" "#, None)
                    ),
                    @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"},{"json":"{\"a\":{\"b\":[[1,2]]}}"}]}}"###
                );
            }
            _ => {
                insta::assert_snapshot!(
                    run_query!(
                        runner,
                        jsonq(&runner, r#"array_contains: "[[1, 2]]" "#, None)
                    ),
                    @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[[1,2]]}}"}]}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn array_contains_pg_json(runner: Runner) -> TestResult<()> {
        array_contains_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn array_contains(runner: Runner) -> TestResult<()> {
        array_contains_runner(runner).await?;

        Ok(())
    }

    async fn array_starts_with_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"3"#, true).await?;
        create_row(&runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 5, r#"\"a\""#, true).await?;
        create_row(&runner, 6, r#"[[1, 2]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[3,4,5]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "\"a\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[\"a\",\"b\"]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_starts_with: "[1, 2]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[[1,2]]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn array_starts_with_pg_json(runner: Runner) -> TestResult<()> {
        array_starts_with_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn array_starts_with(runner: Runner) -> TestResult<()> {
        array_starts_with_runner(runner).await?;

        Ok(())
    }

    async fn array_ends_with_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 4, r#"[[1, 2], [3, 4]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[\"a\",\"b\"]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"array_ends_with: "[3, 4]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[[1,2],[3,4]]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn array_ends_with_pg_json(runner: Runner) -> TestResult<()> {
        array_ends_with_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn array_ends_with(runner: Runner) -> TestResult<()> {
        array_ends_with_runner(runner).await?;

        Ok(())
    }

    async fn string_contains_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"fool\""#, true).await?;
        create_row(&runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_contains: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"fool\"}}"}]}}"###
        );
        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn string_contains_pg_json(runner: Runner) -> TestResult<()> {
        string_contains_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn string_contains(runner: Runner) -> TestResult<()> {
        string_contains_runner(runner).await?;

        Ok(())
    }

    async fn string_starts_with_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"fool\""#, true).await?;
        create_row(&runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_starts_with: "foo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"fool\"}}"}]}}"###
        );
        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn string_starts_with_pg_json(runner: Runner) -> TestResult<()> {
        string_starts_with_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn string_starts_with(runner: Runner) -> TestResult<()> {
        string_starts_with_runner(runner).await?;

        Ok(())
    }

    async fn string_ends_with_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"\"foo\""#, true).await?;
        create_row(&runner, 2, r#"\"fool\""#, true).await?;
        create_row(&runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_ends_with: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"}]}}"###
        );
        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn string_ends_with_pg_json(runner: Runner) -> TestResult<()> {
        string_ends_with_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn string_ends_with(runner: Runner) -> TestResult<()> {
        string_ends_with_runner(runner).await?;

        Ok(())
    }

    async fn gt_gte_runner(runner: Runner) -> TestResult<()> {
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
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"bar\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gte: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"bar\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gt: "1" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":2}}"},{"json":"{\"a\":{\"b\":1.4}}"},{"json":"{\"a\":{\"b\":100}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"gte: "1" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":1}}"},{"json":"{\"a\":{\"b\":2}}"},{"json":"{\"a\":{\"b\":1.4}}"},{"json":"{\"a\":{\"b\":100}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn gt_gte_pg_json(runner: Runner) -> TestResult<()> {
        gt_gte_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn gt_gte(runner: Runner) -> TestResult<()> {
        gt_gte_runner(runner).await?;

        Ok(())
    }

    async fn lt_lte_runner(runner: Runner) -> TestResult<()> {
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
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"bar\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lte: "\"foo\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"bar\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lt: "100" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":1}}"},{"json":"{\"a\":{\"b\":2}}"},{"json":"{\"a\":{\"b\":1.4}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"lte: "100" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":1}}"},{"json":"{\"a\":{\"b\":2}}"},{"json":"{\"a\":{\"b\":1.4}}"},{"json":"{\"a\":{\"b\":100}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn lt_lte_pg_json(runner: Runner) -> TestResult<()> {
        lt_lte_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn lt_lte(runner: Runner) -> TestResult<()> {
        lt_lte_runner(runner).await?;

        Ok(())
    }

    async fn multi_filtering_runner(runner: Runner) -> TestResult<()> {
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

    #[connector_test(schema(pg_json), only(Postgres))]
    async fn multi_filtering_pg_json(runner: Runner) -> TestResult<()> {
        multi_filtering_runner(runner).await?;

        Ok(())
    }

    #[connector_test]
    async fn multi_filtering(runner: Runner) -> TestResult<()> {
        multi_filtering_runner(runner).await?;

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
            r#"query {{ findManyTestModel(where: {{ json: {{ {}, {} }} }} ) {{ json }} }}"#,
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

use query_engine_tests::*;

#[test_suite(schema(schemas::json), capabilities(JsonFilteringJsonPath, JsonFilteringArrayPath))]
mod json_path {
    use query_engine_tests::ConnectorTag;

    #[connector_test]
    async fn no_path_without_filter(runner: &Runner) -> TestResult<()> {
        assert_error!(
            runner,
            jsonq(runner, json_path(runner), Some("")),
            2019,
            "A JSON path cannot be set without a scalar filter."
        );

        Ok(())
    }

    #[connector_test(capabilities(JsonFilteringArrayPath))]
    async fn extract_array_path(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"{ \"a\": { \"b\": \"c\" } }"#, false).await?;
        create_row(runner, 2, r#"{ \"a\": { \"b\": [1, 2, 3] } }"#, false).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"path: ["a", "b"], equals: "\"c\"" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"c\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"path: ["a", "b", "0"], equals: "1" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test(capabilities(JsonFilteringJsonPath), exclude(MySql(5.6)))]
    async fn extract_json_path(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"{ \"a\": { \"b\": \"c\" } }"#, false).await?;
        create_row(runner, 2, r#"{ \"a\": { \"b\": [1, 2, 3] } }"#, false).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"path: "$.a.b", equals: "\"c\"" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"c\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"path: "$.a.b[0]", equals: "1" "#, Some(""))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn array_contains(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(runner, 3, r#"3"#, true).await?;
        create_row(runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(runner, 5, r#"\"a\""#, true).await?;
        create_row(runner, 6, r#"[[1, 2]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_contains: "[3]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"},{"json":"{\"a\":{\"b\":[3,4,5]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_contains: "[\"a\"]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[\"a\",\"b\"]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_contains: "[[1, 2]]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[[1,2]]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn array_starts_with(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(runner, 3, r#"3"#, true).await?;
        create_row(runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(runner, 5, r#"\"a\""#, true).await?;
        create_row(runner, 6, r#"[[1, 2]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_starts_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[3,4,5]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_starts_with: "\"a\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[\"a\",\"b\"]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_starts_with: "[1, 2]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[[1,2]]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn array_ends_with(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(runner, 3, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(runner, 4, r#"[[1, 2], [3, 4]]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_ends_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[1,2,3]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_ends_with: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[\"a\",\"b\"]}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"array_ends_with: "[3, 4]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[[1,2],[3,4]]}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn string_contains(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"\"foo\""#, true).await?;
        create_row(runner, 2, r#"\"fool\""#, true).await?;
        create_row(runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"string_contains: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"fool\"}}"}]}}"###
        );
        Ok(())
    }

    #[connector_test]
    async fn string_starts_with(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"\"foo\""#, true).await?;
        create_row(runner, 2, r#"\"fool\""#, true).await?;
        create_row(runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"string_starts_with: "foo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"fool\"}}"}]}}"###
        );
        Ok(())
    }

    #[connector_test]
    async fn string_ends_with(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"\"foo\""#, true).await?;
        create_row(runner, 2, r#"\"fool\""#, true).await?;
        create_row(runner, 3, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"string_ends_with: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"}]}}"###
        );
        Ok(())
    }

    #[connector_test]
    async fn gt_gte(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"\"foo\""#, true).await?;
        create_row(runner, 2, r#"\"bar\""#, true).await?;
        create_row(runner, 3, r#"1"#, true).await?;
        create_row(runner, 4, r#"2"#, true).await?;
        create_row(runner, 5, r#"1.4"#, true).await?;
        create_row(runner, 6, r#"[\"foo\"]"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"gt: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"bar\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"gte: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":\"foo\"}}"},{"json":"{\"a\":{\"b\":\"bar\"}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"gt: "1" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":2}}"},{"json":"{\"a\":{\"b\":1.4}}"}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(runner, r#"gte: "1" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":1}}"},{"json":"{\"a\":{\"b\":2}}"},{"json":"{\"a\":{\"b\":1.4}}"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn multi_filtering(runner: &Runner) -> TestResult<()> {
        create_row(runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(runner, 3, r#"[3, 4, 6]"#, true).await?;
        create_row(runner, 4, r#"[5, 6, 7]"#, true).await?;
        create_row(runner, 5, r#"1"#, true).await?;
        create_row(runner, 6, r#"2.4"#, true).await?;
        create_row(runner, 7, r#"3"#, true).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                format!(r#"query {{
                    findManyTestModel(
                        where: {{ json: {{ {}, array_contains: "3", array_starts_with: "3" }} }},
                        cursor: {{ id: 2 }},
                        take: 2
                    ) {{ json }}
                }}"#, json_path(runner))
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
                }}"#, json_path(runner), json_path(runner))
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
            r#"query {{ findManyTestModel(where: {{ json: {{ {}, {} }} }} ) {{ json }} }}"#,
            filter, path
        )
    }

    fn json_path(runner: &Runner) -> &str {
        match runner.connector() {
            ConnectorTag::Postgres(_) => r#"path: ["a", "b"]"#,
            ConnectorTag::MySql(_) => r#"path: "$.a.b""#,
            x => unreachable!("JSON filtering is not supported on {}", x),
        }
    }
}

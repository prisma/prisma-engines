use query_engine_tests::*;

#[test_suite(schema(schemas::json), capabilities(JsonFiltering), exclude(MySql(5.6)))]
mod json_filters {
    use indoc::indoc;
    use query_engine_tests::{assert_error, is_one_of, run_query, MySqlVersion, Runner};

    fn pg_json() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                json Json @test.Json
            }"#
        };

        schema.to_owned()
    }

    fn cdb_json() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                json Json @test.JsonB
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

    #[connector_test(schema(pg_json), capabilities(JsonFilteringArrayPath), exclude(CockroachDb))]
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

    async fn array_contains_runner(runner: Runner) -> TestResult<()> {
        create_row(&runner, 1, r#"[1, 2, 3]"#, true).await?;
        create_row(&runner, 2, r#"[3, 4, 5]"#, true).await?;
        create_row(&runner, 3, r#"3"#, true).await?;
        create_row(&runner, 4, r#"[\"a\", \"b\"]"#, true).await?;
        create_row(&runner, 5, r#"\"a\""#, true).await?;
        create_row(&runner, 6, r#"[[1, 2]]"#, true).await?;
        create_row(&runner, 7, r#"[1, null, 2]"#, true).await?;
        create_row(&runner, 8, r#"[1, [null], 2]"#, true).await?;

        // array_contains
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

        // NOT array_contains
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_contains: "[3]""#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":4},{"id":6},{"id":7},{"id":8}]}}"###
        );
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_contains: "[\"a\"]""#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":7},{"id":8}]}}"###
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

        match runner.connector_version() {
            // MariaDB does not support finding arrays in arrays, unlike MySQL
            ConnectorVersion::MySql(Some(MySqlVersion::MariaDb)) => {
                insta::assert_snapshot!(
                    run_query!(
                        runner,
                        jsonq(&runner, r#"array_contains: "[[1, 2]]" "#, None)
                    ),
                    @r###"{"data":{"findManyTestModel":[{"id":1},{"id":6},{"id":7},{"id":8}]}}"###
                );
            }
            _ => {
                insta::assert_snapshot!(
                    run_query!(
                        runner,
                        jsonq(&runner, r#"array_contains: "[[1, 2]]" "#, None)
                    ),
                    @r###"{"data":{"findManyTestModel":[{"id":6}]}}"###
                );

                insta::assert_snapshot!(
                    run_query!(
                        runner,
                        jsonq(&runner, r#"array_contains: "[[null]]" "#, None)
                    ),
                    @r###"{"data":{"findManyTestModel":[{"id":8}]}}"###
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

        // NOT
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_starts_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":4},{"id":6},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_starts_with: "\"a\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":6},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_starts_with: "[1, 2]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_starts_with: "null" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":6},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_starts_with: "[null]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":6},{"id":8}]}}"###
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

        // NOT
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_ends_with: "3" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":2},{"id":3},{"id":4},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_ends_with: "\"b\"" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":4},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_ends_with: "[3, 4]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":8},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_ends_with: "null" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":9}]}}"###
        );

        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"array_ends_with: "[null]" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":8}]}}"###
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
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // NOT
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"string_contains: "ab" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
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

        // string_starts_with
        insta::assert_snapshot!(
            run_query!(
                runner,
                jsonq(&runner, r#"string_starts_with: "foo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
        );

        // NOT string_starts_with
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"string_starts_with: "ab" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
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
            @r###"{"data":{"findManyTestModel":[{"id":1}]}}"###
        );

        // NOT
        insta::assert_snapshot!(
            run_query!(
                runner,
                not_jsonq(&runner, r#"string_ends_with: "oo" "#, None)
            ),
            @r###"{"data":{"findManyTestModel":[{"id":2}]}}"###
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

    // CockroachDB does not support JSON comparisons (https://github.com/cockroachdb/cockroach/issues/49144).
    #[connector_test(schema(pg_json), only(Postgres), exclude(CockroachDb))]
    async fn gt_gte_pg_json(runner: Runner) -> TestResult<()> {
        gt_gte_runner(runner).await?;

        Ok(())
    }

    #[connector_test(schema(cdb_json), only(CockroachDb))]
    async fn cockroach_errors_on_json_gt_lt(runner: Runner) -> TestResult<()> {
        let query = format!(
            r#"query {{
            findManyTestModel(
                where: {{
                    AND: [
                        {{ json: {{ {}, gte: "1" }} }},
                        {{ json: {{ {}, lt: "3" }} }},
                    ]
                }}
            ) {{ json }}
        }}"#,
            json_path(&runner),
            json_path(&runner)
        );

        assert_error!(&runner, query, 2009);
        Ok(())
    }

    // CockroachDB does not support JSON comparisons (https://github.com/cockroachdb/cockroach/issues/49144).
    #[connector_test(only(Postgres), exclude(CockroachDb))]
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

    // CockroachDB does not support JSON comparisons (https://github.com/cockroachdb/cockroach/issues/49144).
    #[connector_test(schema(pg_json), only(Postgres), exclude(CockroachDb))]
    async fn lt_lte_pg_json(runner: Runner) -> TestResult<()> {
        lt_lte_runner(runner).await?;

        Ok(())
    }

    // CockroachDB does not support JSON comparisons (https://github.com/cockroachdb/cockroach/issues/49144).
    #[connector_test(only(Postgres), exclude(CockroachDb))]
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

        // NOT
        insta::assert_snapshot!(
            run_query!(
                runner,
                format!(r#"query {{
                    findManyTestModel(
                        where: {{ NOT: {{ json: {{ {}, array_contains: "3", array_starts_with: "3" }} }} }},
                        cursor: {{ id: 2 }},
                        take: 2
                    ) {{ json }}
                }}"#, json_path(&runner))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":[5,6,7]}}"}]}}"###
        );
        // 1, 2.4, 3
        // filter: false, true, false
        // negated: true, false, true
        // result: 1, 3
        insta::assert_snapshot!(
            run_query!(
                runner,
                format!(r#"query {{
                    findManyTestModel(
                        where: {{
                            NOT: {{ AND: [
                                    {{ json: {{ {}, gt: "1" }} }},
                                    {{ json: {{ {}, lt: "3" }} }},
                            ]}}
                        }}
                    ) {{ json }}
                }}"#, json_path(&runner), json_path(&runner))
            ),
            @r###"{"data":{"findManyTestModel":[{"json":"{\"a\":{\"b\":1}}"},{"json":"{\"a\":{\"b\":3}}"}]}}"###
        );

        Ok(())
    }

    // CockroachDB does not support JSON comparisons (https://github.com/cockroachdb/cockroach/issues/49144).
    #[connector_test(schema(pg_json), only(Postgres), exclude(CockroachDb))]
    async fn multi_filtering_pg_json(runner: Runner) -> TestResult<()> {
        multi_filtering_runner(runner).await?;

        Ok(())
    }

    // CockroachDB does not support JSON comparisons (https://github.com/cockroachdb/cockroach/issues/49144).
    #[connector_test(only(Postgres), exclude(CockroachDb))]
    async fn multi_filtering(runner: Runner) -> TestResult<()> {
        multi_filtering_runner(runner).await?;

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn string_contains_does_not_error(runner: Runner) -> TestResult<()> {
        // NOTE: with string operations the results are always empty because we check for an object, not a string
        // in any case, this should not fail, it will work and return an empty result
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findFirstTestModel(where: { json: { string_contains: "foo" }  }) { id }}"#),
            @r###"{"data":{"findFirstTestModel":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn string_begins_with_does_not_error(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findFirstTestModel(where: { json: { string_starts_with: "foo" }  }) { id }}"#),
            @r###"{"data":{"findFirstTestModel":null}}"###
        );

        Ok(())
    }

    #[connector_test(schema(json_opt))]
    async fn string_ends_with_does_not_error(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findFirstTestModel(where: { json: { string_ends_with: "foo" }  }) { id }}"#),
            @r###"{"data":{"findFirstTestModel":null}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, id: u32, data: &str, nested: bool) -> TestResult<()> {
        let json = if nested {
            format!(r#"{{ \"a\": {{ \"b\": {data} }} }}"#)
        } else {
            data.to_owned()
        };
        let q = format!(r#"mutation {{ createOneTestModel(data: {{ id: {id}, json: "{json}" }}) {{ id }} }}"#);

        runner.query(q).await?.assert_success();
        Ok(())
    }

    fn jsonq(runner: &Runner, filter: &str, path: Option<&str>) -> String {
        let path = path.unwrap_or_else(|| json_path(runner));

        format!(r#"query {{ findManyTestModel(where: {{ json: {{ {filter}, {path} }} }} ) {{ id }} }}"#)
    }

    fn not_jsonq(runner: &Runner, filter: &str, path: Option<&str>) -> String {
        let path = path.unwrap_or_else(|| json_path(runner));

        format!(r#"query {{ findManyTestModel(where: {{ NOT: {{ json: {{ {filter}, {path} }} }} }} ) {{ id }} }}"#)
    }

    fn json_path(runner: &Runner) -> &'static str {
        match runner.connector_version() {
            ConnectorVersion::Postgres(_) | ConnectorVersion::CockroachDb => r#"path: ["a", "b"]"#,
            ConnectorVersion::MySql(_) => r#"path: "$.a.b""#,
            x => unreachable!("JSON filtering is not supported on {:?}", x),
        }
    }
}

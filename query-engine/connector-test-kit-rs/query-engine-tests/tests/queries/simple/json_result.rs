use query_engine_tests::*;

#[test_suite(schema(json), capabilities(Json), exclude(MySQL(5.6)))]
mod json_as_result {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn test_when_distinct_result_is_json(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneTestModel(data: {id: 101 json: "{\"foo\": 1}"}){id} }"#)
            .await?
            .assert_success();

        assert_query!(
            runner,
            "query {findFirstTestModel(distinct: [json]) {json}}",
            r#"{"data":{"findFirstTestModel":{"json":"{\"foo\":1}"}}}"#
        );

        Ok(())
    }
}

use query_engine_tests::*;

#[test_suite(schema(json), capabilities(Json))]
mod json_as_result {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn test_when_distinct_result_is_json(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneTestModel(data: {id: 101 json: "{\"foo\": 1}"}){id} }"#)
            .await?
            .assert_success();

        assert_eq!(
            runner
                .query("query {findFirstTestModel(distinct: [json]) {json}}")
                .await?
                .to_string()
                .replace(" ", ""), // ignore whitespace in the JSON string
            r#"{"data":{"findFirstTestModel":{"json":"{\"foo\":1}"}}}"#
        );

        Ok(())
    }
}

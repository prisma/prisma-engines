use query_engine_tests::*;

#[test_suite(schema(json_opt), capabilities(Json), exclude(MySql(5.6)))]
mod json {
    use query_engine_tests::{run_query, EngineProtocol, Runner};

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { json } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"json":"{}"}}}"###
                );
            }
            EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                            "modelName": "TestModel",
                            "action": "findUnique",
                            "query": {
                                "arguments": {
                                    "where": { "id": 1 }
                                },
                                "selection": {
                                    "json": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"json":{"$type":"Json","value":"{}"}}}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn read_many(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findManyTestModel { json } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"json":"{}"},{"json":"{\"a\":\"b\"}"},{"json":"1"},{"json":"\"hello\""},{"json":"[1,\"a\",{\"b\":true}]"}]}}"###
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                    "modelName": "TestModel",
                    "action": "findMany",
                    "query": {
                        "selection": {
                            "json": true
                        }
                    }
                }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"json":{"$type":"Json","value":"{}"}},{"json":{"$type":"Json","value":"{\"a\":\"b\"}"}},{"json":{"$type":"Json","value":"1"}},{"json":{"$type":"Json","value":"\"hello\""}},{"json":{"$type":"Json","value":"[1,\"a\",{\"b\":true}]"}}]}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn json_null(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: DbNull }"#).await?;
        create_row(&runner, r#"{ id: 2, json: JsonNull }"#).await?;

        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findManyTestModel { json } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"json":null},{"json":"null"}]}}"###
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                            "modelName": "TestModel",
                            "action": "findMany",
                            "query": {
                                "selection": {
                                    "json": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"json":null},{"json":{"$type":"Json","value":"null"}}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: "{}" }"#).await?;
        create_row(&runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(&runner, r#"{ id: 3, json: "1" }"#).await?;
        create_row(&runner, r#"{ id: 4, json: "\"hello\"" }"#).await?;
        create_row(&runner, r#"{ id: 5, json: "[1, \"a\", {\"b\": true}]" }"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}

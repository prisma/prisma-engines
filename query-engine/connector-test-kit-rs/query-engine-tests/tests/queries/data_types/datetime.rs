use query_engine_tests::*;

#[test_suite(schema(common_nullable_types))]
mod datetime {
    use query_engine_tests::{run_query, EngineProtocol, Runner};

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { dt } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"dt":"1900-10-10T01:10:10.001Z"}}}"###
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
                                    "dt": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"dt":{"$type":"DateTime","value":"1900-10-10T01:10:10.001Z"}}}}"###
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
                let res = run_query!(runner, r#"{ findManyTestModel { dt } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"dt":"1900-10-10T01:10:10.001Z"},{"dt":"1969-01-01T10:33:59.000Z"},{"dt":null}]}}"###
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
                                "dt": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"dt":{"$type":"DateTime","value":"1900-10-10T01:10:10.001Z"}},{"dt":{"$type":"DateTime","value":"1969-01-01T10:33:59.000Z"}},{"dt":null}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, dt: "1900-10-10T01:10:10.001Z" }"#).await?;
        create_row(runner, r#"{ id: 2, dt: "1969-01-01T10:33:59.000Z" }"#).await?;
        create_row(runner, r#"{ id: 3 }"#).await?;

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

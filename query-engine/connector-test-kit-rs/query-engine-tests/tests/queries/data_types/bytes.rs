use query_engine_tests::*;

#[test_suite(schema(common_nullable_types))]
mod bytes {
    use query_engine_tests::{run_query, EngineProtocol, Runner};

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { bytes } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"bytes":"AQID"}}}"###
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
                                    "bytes": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"bytes":{"$type":"Bytes","$value":"AQID"}}}}"###
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
                let res = run_query!(runner, r#"{ findManyTestModel { bytes } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"bytes":"AQID"},{"bytes":"dGVzdA=="},{"bytes":null}]}}"###
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
                                    "bytes": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"bytes":{"$type":"Bytes","$value":"AQID"}},{"bytes":{"$type":"Bytes","$value":"dGVzdA=="}},{"bytes":null}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, bytes: "AQID" }"#).await?;
        create_row(runner, r#"{ id: 2, bytes: "dGVzdA==" }"#).await?;
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

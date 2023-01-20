use query_engine_tests::*;

#[test_suite(schema(common_nullable_types))]
mod bool {
    use query_engine_tests::{run_query, EngineProtocol, Runner};

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { bool } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"bool":"10000000000"}}}"###
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
                                    "bool": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"bool":true}}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn read_many(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findManyTestModel { bool } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"bool":"10000000000"},{"bool":"-10000000000"},{"bool":null}]}}"###
                );
            }
            EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                        "modelName": "TestModel",
                        "action": "findMany",
                        "query": {
                            "selection": {
                                "bool": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"bool":true},{"bool":false},{"bool":null}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, bool: true }"#).await?;
        create_row(runner, r#"{ id: 2, bool: false }"#).await?;
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

use query_engine_tests::*;

#[test_suite(schema(common_nullable_types))]
mod int {
    use query_engine_tests::Runner;

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { int } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"int":"10000000000"}}}"###
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
                                    "int": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"int":-42}}}"###
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
                let res = run_query!(runner, r#"{ findManyTestModel { int } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"int":"10000000000"},{"int":"-10000000000"},{"int":null}]}}"###
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
                                "int": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"int":-42},{"int":1337},{"int":null}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, int: -42 }"#).await?;
        create_row(runner, r#"{ id: 2, int: 1337 }"#).await?;
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

use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(Enums))]
mod enum_type {
    use query_engine_tests::Runner;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)
                my_enum MyEnum?
            }

            enum MyEnum {
                A
                B
                C
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { my_enum } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"my_enum":"10000000000"}}}"###
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
                                    "my_enum": true
                                }
                            }
                        }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findUniqueTestModel":{"my_enum":"A"}}}"###
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
                let res = run_query!(runner, r#"{ findManyTestModel { my_enum } }"#);

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"my_enum":"10000000000"},{"my_enum":"-10000000000"},{"my_enum":null}]}}"###
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
                                "my_enum": true
                            }
                        }
                    }"#,
                    )
                    .await?;

                insta::assert_snapshot!(
                  res.to_string(),
                  @r###"{"data":{"findManyTestModel":[{"my_enum":"A"},{"my_enum":"B"},{"my_enum":null}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, my_enum: A }"#).await?;
        create_row(runner, r#"{ id: 2, my_enum: B }"#).await?;
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

use query_engine_tests::*;

#[test_suite(schema(common_nullable_types))]
mod bytes {
    use query_engine_tests::{run_query, EngineProtocol, Runner};

    #[test_suite]
    mod issue_687 {
        fn schema_common() -> String {
            let schema = indoc! {
                r#"model Parent {
                    #id(id, Int, @id)
            
                    children Child[]
                }
                
                model Child {
                    #id(childId, Int, @id)
            
                    parentId Int?
                    parent Parent? @relation(fields: [parentId], references: [id])
            
                    bytes   Bytes
                }
            "#
            };

            schema.to_owned()
        }

        async fn create_common_children(runner: &Runner) -> TestResult<()> {
            create_child(
                runner,
                r#"{
                    childId: 1,
                    bytes: "AQID",
                }"#,
            )
            .await?;

            create_child(
                runner,
                r#"{
                    childId: 2,
                    bytes: "FDSF"
                }"#,
            )
            .await?;

            create_parent(
                runner,
                r#"{ id: 1, children: { connect: [{ childId: 1 }, { childId: 2 }] } }"#,
            )
            .await?;

            Ok(())
        }

        #[connector_test(schema(schema_common))]
        async fn common_types(runner: Runner) -> TestResult<()> {
            create_common_children(&runner).await?;

            insta::assert_snapshot!(
              run_query!(&runner, r#"{ findManyParent { id children { childId bytes } } }"#),
              @r###"{"data":{"findManyParent":[{"id":1,"children":[{"childId":1,"bytes":"AQID"},{"childId":2,"bytes":"FDSF"}]}]}}"###
            );

            Ok(())
        }

        async fn create_child(runner: &Runner, data: &str) -> TestResult<()> {
            runner
                .query(format!("mutation {{ createOneChild(data: {}) {{ childId }} }}", data))
                .await?
                .assert_success();
            Ok(())
        }

        async fn create_parent(runner: &Runner, data: &str) -> TestResult<()> {
            runner
                .query(format!("mutation {{ createOneParent(data: {}) {{ id }} }}", data))
                .await?
                .assert_success();
            Ok(())
        }
    }

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { bytes } }"#);

                insta::assert_snapshot!(
                  res,
                  @r###"{"data":{"findUniqueTestModel":{"bytes":"FSDF"}}}"###
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
                  @r###"{"data":{"findUniqueTestModel":{"bytes":{"$type":"Bytes","value":"FSDF"}}}}"###
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
                  res,
                  @r###"{"data":{"findManyTestModel":[{"bytes":"FSDF"},{"bytes":"dGVzdA=="},{"bytes":null}]}}"###
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
                  @r###"{"data":{"findManyTestModel":[{"bytes":{"$type":"Bytes","value":"FSDF"}},{"bytes":{"$type":"Bytes","value":"dGVzdA=="}},{"bytes":null}]}}"###
                );
            }
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, bytes: "FSDF" }"#).await?;
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

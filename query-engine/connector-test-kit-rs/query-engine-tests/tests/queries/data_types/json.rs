use query_engine_tests::*;

#[test_suite(schema(json_opt), capabilities(Json))]
mod json {
    use query_engine_tests::{EngineProtocol, Runner, run_query};

    #[connector_test]
    async fn read_one(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 1 }) { json } }"#);

                insta::assert_snapshot!(
                  res,
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
                  res.replace(" ", ""), // ignore whitespace in the JSON string
                  @r###"{"data":{"findManyTestModel":[{"json":"{}"},{"json":"{\"a\":\"b\"}"},{"json":"1"},{"json":"1.5"},{"json":"\"hello\""},{"json":"[1,\"a\",{\"b\":true}]"},{"json":"true"}]}}"###
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
                  res.to_string().replace(" ", ""), // ignore whitespace in the JSON string
                  @r###"{"data":{"findManyTestModel":[{"json":{"$type":"Json","value":"{}"}},{"json":{"$type":"Json","value":"{\"a\":\"b\"}"}},{"json":{"$type":"Json","value":"1"}},{"json":{"$type":"Json","value":"1.5"}},{"json":{"$type":"Json","value":"\"hello\""}},{"json":{"$type":"Json","value":"[1,\"a\",{\"b\":true}]"}},{"json":{"$type":"Json","value":"true"}}]}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn read_plain_float(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 4 }) { json } }"#);

                insta::assert_snapshot!(
                  res,
                  @r###"{"data":{"findUniqueTestModel":{"json":"1.5"}}}"###
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                    "modelName": "TestModel",
                    "action": "findUnique",
                    "query": {
                        "arguments": {
                            "where": { "id": 4 }
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
                  @r###"{"data":{"findUniqueTestModel":{"json":{"$type":"Json","value":"1.5"}}}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn read_plain_int(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 3 }) { json } }"#);

                insta::assert_snapshot!(
                  res,
                  @r###"{"data":{"findUniqueTestModel":{"json":"1"}}}"###
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                    "modelName": "TestModel",
                    "action": "findUnique",
                    "query": {
                        "arguments": {
                            "where": { "id": 3 }
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
                  @r###"{"data":{"findUniqueTestModel":{"json":{"$type":"Json","value":"1"}}}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn read_plain_bool(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findUniqueTestModel(where: { id: 7 }) { json } }"#);

                insta::assert_snapshot!(
                  res,
                  @r###"{"data":{"findUniqueTestModel":{"json":"true"}}}"###
                );
            }
            query_engine_tests::EngineProtocol::Json => {
                let res = runner
                    .query_json(
                        r#"{
                    "modelName": "TestModel",
                    "action": "findUnique",
                    "query": {
                        "arguments": {
                            "where": { "id": 7 }
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
                  @r###"{"data":{"findUniqueTestModel":{"json":{"$type":"Json","value":"true"}}}}"###
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
                  res,
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

    #[connector_test(capabilities(AdvancedJsonNullability))]
    async fn json_null_must_not_be_confused_with_literal_string(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, json: "\"null\"" }"#).await?;

        match runner.protocol() {
            query_engine_tests::EngineProtocol::Graphql => {
                let res = run_query!(runner, r#"{ findManyTestModel { json } }"#);

                insta::assert_snapshot!(
                  res,
                  @r###"{"data":{"findManyTestModel":[{"json":"\"null\""}]}}"###
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
                  @r###"{"data":{"findManyTestModel":[{"json":{"$type":"Json","value":"\"null\""}}]}}"###
                );
            }
        }

        Ok(())
    }

    #[connector_test]
    async fn dollar_type_in_json_protocol(runner: Runner) -> TestResult<()> {
        let res = runner
            .query_json(
                r#"{
                    "modelName": "TestModel",
                    "action": "createOne",
                    "query": {
                       "selection": { "json": true },
                       "arguments": {
                          "data": {
                             "id": 1,
                             "json": { "$type": "Raw", "value": {"$type": "Something" } }
                          }
                       }
                    }
                }"#,
            )
            .await?;

        res.assert_success();

        insta::assert_snapshot!(res.to_string(), @r###"{"data":{"createOneTestModel":{"json":{"$type":"Json","value":"{\"$type\":\"Something\"}"}}}}"###);

        Ok(())
    }

    #[connector_test]
    async fn nested_dollar_type_in_json_protocol(runner: Runner) -> TestResult<()> {
        let res = runner
            .query_json(
                r#"{
                    "modelName": "TestModel",
                    "action": "createOne",
                    "query": {
                       "selection": { "json": true },
                       "arguments": {
                          "data": {
                             "id": 1,
                             "json": {
                                "something": { "$type": "Raw", "value": {"$type": "Something" } }
                             }
                          }
                       }
                    }
                }"#,
            )
            .await?;

        res.assert_success();

        insta::assert_snapshot!(res.to_string(), @r###"{"data":{"createOneTestModel":{"json":{"$type":"Json","value":"{\"something\":{\"$type\":\"Something\"}}"}}}}"###);

        Ok(())
    }

    fn schema_json_list() -> String {
        let schema = indoc! {
            r#"model TestModel {
                #id(id, Int, @id)

                child Child?
            }

            model Child {
                #id(id, Int, @id)
                json_list Json[]

                testId Int? @unique
                test   TestModel? @relation(fields: [testId], references: [id])
            }"#
        };

        schema.to_owned()
    }

    #[connector_test(schema(schema_json_list), capabilities(Json, ScalarLists), exclude(CockroachDb))]
    async fn json_list(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, child: { create: { id: 1, json_list: ["1", "2"] } } }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 2, child: { create: { id: 2, json_list: ["{}"] } } }"#).await?;
        create_row(
            &runner,
            r#"{ id: 3, child: { create: { id: 3, json_list: ["\"hello\"", "\"world\""] } } }"#,
        )
        .await?;
        create_row(&runner, r#"{ id: 4, child: { create: { id: 4 } } }"#).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { child { json_list } } }"#),
          @r###"{"data":{"findManyTestModel":[{"child":{"json_list":["1","2"]}},{"child":{"json_list":["{}"]}},{"child":{"json_list":["\"hello\"","\"world\""]}},{"child":{"json_list":[]}}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, json: "{}" }"#).await?;
        create_row(runner, r#"{ id: 2, json: "{\"a\":\"b\"}" }"#).await?;
        create_row(runner, r#"{ id: 3, json: "1" }"#).await?;
        create_row(runner, r#"{ id: 4, json: "1.5" }"#).await?;
        create_row(runner, r#"{ id: 5, json: "\"hello\"" }"#).await?;
        create_row(runner, r#"{ id: 6, json: "[1, \"a\", {\"b\": true}]" }"#).await?;
        create_row(runner, r#"{ id: 7, json: "true" }"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}

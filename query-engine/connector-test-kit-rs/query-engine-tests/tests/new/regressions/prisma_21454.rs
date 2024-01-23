use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(Json))]
mod prisma_21454 {

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Model {
                #id(id, String, @id)
                json Json
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn dollar_type_in_json(runner: Runner) -> TestResult<()> {
        let res = runner
            .query_json(
                r#"{
                    "modelName": "Model",
                    "action": "createOne",
                    "query": {
                       "selection": { "json": true },
                       "arguments": {
                          "data": {
                             "id": "123",
                             "json": { "$type": "Json", "value": "{\"$type\": \"Something\" }" }
                          }
                       }
                    }
                }"#,
            )
            .await?;

        res.assert_success();

        insta::assert_snapshot!(res.to_string(), @r###"{"data":{"createOneModel":{"json":{"$type":"Json","value":"{\"$type\":\"Something\"}"}}}}"###);

        Ok(())
    }

    #[connector_test]
    async fn nested_dollar_type_in_json(runner: Runner) -> TestResult<()> {
        let res = runner
            .query_json(
                r#"{
                    "modelName": "Model",
                    "action": "createOne",
                    "query": {
                       "selection": { "json": true },
                       "arguments": {
                          "data": {
                             "id": "123",
                             "json": {
                                "something": { "$type": "Json", "value": "{\"$type\": \"Something\" }" }
                             }
                          }
                       }
                    }
                }"#,
            )
            .await?;

        res.assert_success();

        insta::assert_snapshot!(res.to_string(), @r###"{"data":{"createOneModel":{"json":{"$type":"Json","value":"{\"something\":{\"$type\":\"Something\"}}"}}}}"###);

        Ok(())
    }
}

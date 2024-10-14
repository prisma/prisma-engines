use query_engine_tests::*;

#[test_suite(schema(geom))]
mod json {

    #[connector_test]
    async fn geometry_json_protocol(runner: Runner) -> TestResult<()> {
        let res = runner
            .query_json(
                r#"{
                    "modelName": "TestModel",
                    "action": "createOne",
                    "query": {
                    "selection": { "geom": true },
                    "arguments": {
                        "data": {
                            "id": 1,
                            "geom": { "$type": "Raw", "value": {"type": "Point", "coordinates": [0, 0] } }
                        }
                    }
                    }
                }"#,
            )
            .await?;

        res.assert_success();

        insta::assert_snapshot!(res.to_string(), @r###"{"data":{"createOneTestModel":{"geom":{"$type":"Json","value":"{\"type\":\"Point\",\"coordinates\":[0,0]}"}}}}"###);

        Ok(())
    }
}

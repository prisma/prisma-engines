use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod raw_mongo {
    use indoc::indoc;
    use query_engine_tests::{fmt_execute_raw, fmt_query_raw, run_query, run_query_json, Runner};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              field       String
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn execute_raw(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(
            runner,
            fmt_execute_raw(
                r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }"#,
                vec![]
            )
          ),
          @r###"{"data":{"executeRaw":0}}"###
        );

        insta::assert_snapshot!(
          run_query!(
            runner,
            fmt_execute_raw(
                r#"{ "update": "TestModel", "updates": [{ "q": { "_id": 1 }, "u": { "field": "updated" } }] }"#,
                vec![]
            )
          ),
          @r###"{"data":{"executeRaw":0}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel { id field } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":"updated"},{"id":2,"field":"B"},{"id":3,"field":"C"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_find(runner: Runner) -> TestResult<()> {
        let insertion_res = run_query_json!(
            runner,
            fmt_query_raw(
                r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }"#,
                vec![]
            ),
            &["data", "queryRaw"]
        );

        assert_eq!(
            insertion_res["insertedIds"],
            serde_json::json!({ "0": 1, "1": 2, "2": 3 })
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"{ "find": "TestModel" }"#, vec![])),
          @r###"{"data":{"queryRaw":[{"_id":1,"field":"A"},{"_id":2,"field":"B"},{"_id":3,"field":"C"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_aggregate(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            fmt_query_raw(
                r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }"#,
                vec![]
            )
        );

        // Should consume the cursors before returning the result
        insta::assert_snapshot!(
          run_query!(runner, fmt_query_raw( r#"{ "aggregate": "TestModel", "pipeline": [{ "$project": { "_id": 1 } }] }"#, vec![])),
          @r###"{"data":{"queryRaw":[{"_id":1},{"_id":2},{"_id":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_find_and_modify(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            fmt_query_raw(
                r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A", "age": 1 }, { "_id": 2, "field": "B", "age": 2 }, { "_id": 3, "field": "C", "age": 3 }] }"#,
                vec![]
            )
        );

        // Should fail if neither "remove" or "update" is set
        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw( r#"{ "findAndModify": "TestModel" }"#, vec![])),
          @r###"{"errors":[{"error":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: RawApiError(\"Either an 'update' or 'remove' key must be specified\") })","user_facing_error":{"is_panic":false,"message":"Error occurred during query execution:\nConnectorError(ConnectorError { user_facing_error: None, kind: RawApiError(\"Either an 'update' or 'remove' key must be specified\") })","backtrace":null}}]}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"{ "findAndModify": "TestModel", "query": { "field": "A" }, "update": { "field": "updated" }, "new": true, "fields": { "field": 1 } }"#, vec![])),
          @r###"{"data":{"queryRaw":{"_id":1,"field":"updated"}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_update(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            fmt_query_raw(
                r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }"#,
                vec![]
            )
        );

        // result should not contain $cluster, optTime and similar keys
        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw(r#"{ "update": "TestModel", "updates": [{ "q": { "field": "A" }, "u": { "field": "updated" } }] }"#, vec![])),
          @r###"{"data":{"queryRaw":{"n":1,"nModified":1,"ok":1.0}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_batching(runner: Runner) -> TestResult<()> {
        let res_string = runner.batch(vec![
          fmt_query_raw(
            r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }] }"#,
            vec![]
          ),
          fmt_query_raw(
            r#"{ "insert": "TestModel", "documents": [{ "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }"#,
            vec![]
          )
        ], false).await?.to_string();
        let res_json = serde_json::from_str::<serde_json::Value>(res_string.as_str()).unwrap();
        let res_json = res_json["batchResult"].as_array().expect("Result should be an array");

        let first_query = &res_json[0]["data"]["queryRaw"];
        let second_query = &res_json[1]["data"]["queryRaw"];

        assert_eq!(first_query["insertedIds"], serde_json::json!({ "0": 1 }));
        assert_eq!(second_query["insertedIds"], serde_json::json!({ "0": 2, "1": 3 }));

        Ok(())
    }
}

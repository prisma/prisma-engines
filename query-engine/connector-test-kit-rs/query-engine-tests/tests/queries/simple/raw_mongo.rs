use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod raw_mongo {
    use indoc::indoc;
    use query_engine_tests::{Runner, run_query};
    use serde_json::json;

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
            run_command_raw(
                json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] })
            )
          ),
          @r###"{"data":{"runCommandRaw":{"n":3,"ok":1.0}}}"###
        );

        insta::assert_snapshot!(
          run_query!(
            runner,
            run_command_raw(json!({ "update": "TestModel", "updates": [{ "q": { "_id": 1 }, "u": { "field": "updated" } }] }))
          ),
          @r###"{"data":{"runCommandRaw":{"n":1,"nModified":1,"ok":1.0}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"{ findManyTestModel { id field } }"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":"updated"},{"id":2,"field":"B"},{"id":3,"field":"C"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_find(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            run_command_raw(
                json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] })
            )
        );

        // Should fail if query is not a document
        assert_error!(runner, find_raw(Some(json!([])), None), 0);
        assert_error!(runner, find_raw(Some(json!(1)), None), 0);
        assert_error!(runner, find_raw(Some(json!("a")), None), 0);

        // Should fail if options is not a document
        assert_error!(runner, find_raw(None, Some(json!([]))), 0);
        assert_error!(runner, find_raw(None, Some(json!(1))), 0);
        assert_error!(runner, find_raw(None, Some(json!("a"))), 0);

        // Should work with no query or options
        insta::assert_snapshot!(
          run_query!(&runner, find_raw(None, None)),
          @r###"{"data":{"findTestModelRaw":[{"_id":1,"field":"A"},{"_id":2,"field":"B"},{"_id":3,"field":"C"}]}}"###
        );

        // Should work with a query only
        insta::assert_snapshot!(
          run_query!(&runner, find_raw(Some(json!({ "field": "A" })), None)),
          @r###"{"data":{"findTestModelRaw":[{"_id":1,"field":"A"}]}}"###
        );

        // Should work with options only
        insta::assert_snapshot!(
          run_query!(&runner, find_raw(None, Some(json!({ "skip": 2 })))),
          @r###"{"data":{"findTestModelRaw":[{"_id":3,"field":"C"}]}}"###
        );

        // Should work with a query & options
        insta::assert_snapshot!(
          run_query!(&runner, find_raw(Some(json!({ "field": "A" })), Some(json!({ "skip": 1 })))),
          @r###"{"data":{"findTestModelRaw":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_aggregate(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            run_command_raw(
                json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] })
            )
        );

        // Should fail if pipeline is not an array of document
        assert_error!(
            runner,
            aggregate_raw(Some(vec![json!({ "a": "b" }), json!(2)]), None),
            0
        );

        // Should fail if options is not a document
        assert_error!(runner, aggregate_raw(None, Some(json!([]))), 0);
        assert_error!(runner, aggregate_raw(None, Some(json!(1))), 0);
        assert_error!(runner, aggregate_raw(None, Some(json!("a"))), 0);

        // Should work with no pipeline or options
        insta::assert_snapshot!(
          run_query!(&runner, aggregate_raw(None, None)),
          @r###"{"data":{"aggregateTestModelRaw":[{"_id":1,"field":"A"},{"_id":2,"field":"B"},{"_id":3,"field":"C"}]}}"###
        );

        // Should work with pipeline only
        insta::assert_snapshot!(
          run_query!(runner, aggregate_raw(Some(vec![json!({ "$project": { "_id": 1 } })]), None)),
          @r###"{"data":{"aggregateTestModelRaw":[{"_id":1},{"_id":2},{"_id":3}]}}"###
        );

        // Should work with options only (and not fail on wrong options)
        insta::assert_snapshot!(
          run_query!(&runner, aggregate_raw(None, Some(json!({ "unknown_option": true, "allowDiskUse": true })))),
          @r###"{"data":{"aggregateTestModelRaw":[{"_id":1,"field":"A"},{"_id":2,"field":"B"},{"_id":3,"field":"C"}]}}"###
        );

        // Should work with options & pipeline
        insta::assert_snapshot!(
          run_query!(&runner, aggregate_raw(Some(vec![json!({ "$project": { "_id": 1 } })]), Some(json!({ "unknown_option": true, "allowDiskUse": true })))),
          @r###"{"data":{"aggregateTestModelRaw":[{"_id":1},{"_id":2},{"_id":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_find_and_modify(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            run_command_raw(
                json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A", "age": 1 }, { "_id": 2, "field": "B", "age": 2 }, { "_id": 3, "field": "C", "age": 3 }] })
            )
        );

        // Should fail if neither "remove" or "update" is set
        assert_error!(runner, run_command_raw(json!({ "findAndModify": "TestModel" })), 0);

        insta::assert_snapshot!(
          run_query!(&runner, run_command_raw(json!({ "findAndModify": "TestModel", "query": { "field": "A" }, "update": { "field": "updated" }, "new": true, "fields": { "field": 1 } }))),
          @r###"{"data":{"runCommandRaw":{"lastErrorObject":{"n":1,"updatedExisting":true},"value":{"_id":1,"field":"updated"},"ok":1.0}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_update(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            run_command_raw(
                json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }),
            )
        );

        // result should not contain $cluster, optTime and similar keys
        insta::assert_snapshot!(
          run_query!(&runner, run_command_raw(json!({ "update": "TestModel", "updates": [{ "q": { "field": "A" }, "u": { "field": "updated" } }] }))),
          @r###"{"data":{"runCommandRaw":{"n":1,"nModified":1,"ok":1.0}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn raw_batching(runner: Runner) -> TestResult<()> {
        let res = runner.batch(vec![
          run_command_raw(
            json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }] }),
          ),
          run_command_raw(
            json!({ "insert": "TestModel", "documents": [{ "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }),
          )
        ], false, None).await?.to_string();

        insta::assert_snapshot!(
          res,
          @r###"{"batchResult":[{"data":{"runCommandRaw":{"n":1,"ok":1.0}}},{"data":{"runCommandRaw":{"n":2,"ok":1.0}}}]}"###
        );

        Ok(())
    }

    fn schema_mapped() -> String {
        let schema = indoc! {
            r#"model TestModel {
            #id(id, Int, @id)
            field       String

            @@map("test_model")
          }"#
        };

        schema.to_owned()
    }

    // findRaw & aggregateRaw should work with mapped models
    #[connector_test(schema(schema_mapped))]
    async fn find_aggregate_raw_mapped_model(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            run_command_raw(
                json!({ "insert": "test_model", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }),
            )
        );

        insta::assert_snapshot!(
          run_query!(&runner, aggregate_raw(None, None)),
          @r###"{"data":{"aggregateTestModelRaw":[{"_id":1,"field":"A"},{"_id":2,"field":"B"},{"_id":3,"field":"C"}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, find_raw(None, None)),
          @r###"{"data":{"findTestModelRaw":[{"_id":1,"field":"A"},{"_id":2,"field":"B"},{"_id":3,"field":"C"}]}}"###
        );

        Ok(())
    }

    fn find_raw(filter: Option<serde_json::Value>, options: Option<serde_json::Value>) -> String {
        let filter = filter.map(|q| format!(r#"filter: "{}""#, q.to_string().replace('\"', "\\\"")));
        let options = options.map(|o| format!(r#"options: "{}""#, o.to_string().replace('\"', "\\\"")));

        match (filter, options) {
            (None, None) => r#"query { findTestModelRaw }"#.to_string(),
            (q, o) => {
                format!(
                    r#"query {{ findTestModelRaw({} {}) }}"#,
                    q.unwrap_or_default(),
                    o.unwrap_or_default()
                )
            }
        }
    }

    fn aggregate_raw(pipeline: Option<Vec<serde_json::Value>>, options: Option<serde_json::Value>) -> String {
        let pipeline = pipeline.map(|p| {
            p.into_iter()
                .map(|stage| format!(r#""{}""#, stage.to_string().replace('\"', "\\\"")))
                .collect::<Vec<_>>()
        });
        let pipeline = pipeline.map(|p| format!(r#"pipeline: [{}]"#, p.join(", ")));
        let options = options.map(|o| format!(r#"options: "{}""#, o.to_string().replace('\"', "\\\"")));

        match (pipeline, options) {
            (None, None) => r#"query { aggregateTestModelRaw }"#.to_string(),
            (p, o) => {
                format!(
                    r#"query {{ aggregateTestModelRaw({} {}) }}"#,
                    p.unwrap_or_default(),
                    o.unwrap_or_default()
                )
            }
        }
    }

    fn run_command_raw(command: serde_json::Value) -> String {
        let command = command.to_string().replace('\"', "\\\"");

        format!(r#"mutation {{ runCommandRaw(command: "{command}") }}"#)
    }
}

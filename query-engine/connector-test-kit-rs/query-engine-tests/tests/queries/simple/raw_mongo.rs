use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod raw_mongo {
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
    async fn query_raw(runner: Runner) -> TestResult<()> {
        let insertion_res = run_query_json!(
            runner,
            fmt_query_raw(
                r#"{ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, { "_id": 2, "field": "B" }, { "_id": 3, "field": "C" }] }"#,
                vec![]
            ),
            &["data", "queryRaw"]
        );

        assert_eq!(&insertion_res["ok"].to_string(), "1.0");
        assert_eq!(&insertion_res["n"].to_string(), "3");

        let update_res = run_query_json!(
            runner,
            fmt_query_raw(
                r#"{ "update": "TestModel", "updates": [{ "q": { "_id": 1 }, "u": { "field": "updated" } }] }"#,
                vec![]
            ),
            &["data", "queryRaw"]
        );

        assert_eq!(&update_res["ok"].to_string(), "1.0");
        assert_eq!(&update_res["nModified"].to_string(), "1");

        let find_res = run_query_json!(
            runner,
            fmt_query_raw(r#"{ "find": "TestModel" }"#, vec![]),
            &["data", "queryRaw"]
        );

        assert_eq!(&find_res["ok"].to_string(), "1.0");
        assert_eq!(
            &find_res["cursor"]["firstBatch"].to_string(),
            "[{\"_id\":1,\"field\":\"updated\"},{\"_id\":2,\"field\":\"B\"},{\"_id\":3,\"field\":\"C\"}]"
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

        assert_eq!(&first_query["ok"].to_string(), "1.0");
        assert_eq!(&first_query["n"].to_string(), "1");

        assert_eq!(&second_query["ok"].to_string(), "1.0");
        assert_eq!(&second_query["n"].to_string(), "2");

        Ok(())
    }
}

use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod mongo_incorrect_fields {
    use indoc::indoc;
    use query_engine_tests::{run_query, Runner};
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
    async fn correct_error_for_missing_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(
            runner,
            run_command_raw(
                json!({ "insert": "TestModel", "documents": [{ "_id": 1, "field": "A" }, {"_id": 2}] })
            )
          ),
          @r###"{"data":{"runCommandRaw":{"n":2,"ok":1.0}}}"###
        );

        assert_error!(
            runner,
            "query { findManyTestModel(where: {}) { id, field }}",
            2032,
            "Error converting field \"field\""
        );

        Ok(())
    }

    fn run_command_raw(command: serde_json::Value) -> String {
        let command = command.to_string().replace("\"", "\\\"");

        format!(r#"mutation {{ runCommandRaw(command: "{}") }}"#, command)
    }
}

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod default_value {
    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              composite   Composite
            }
            
            type Composite {
               field     String @default("foo")
               field_opt String? @default("bar")
            }"#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn missing_required_fields_are_backfilled(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            run_command_raw(serde_json::json!({ "insert": "TestModel", "documents": [{ "_id": 1, "composite": {} }] }))
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { composite { field } } }"#),
          @r###"{"data":{"findManyTestModel":[{"composite":{"field":"foo"}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { id: 1 }) { composite { field } } }"#),
          @r###"{"data":{"findUniqueTestModel":{"composite":{"field":"foo"}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn opt_fields_are_not_backfilled(runner: Runner) -> TestResult<()> {
        run_query!(
            runner,
            run_command_raw(serde_json::json!({ "insert": "TestModel", "documents": [{ "_id": 1, "composite": {} }] }))
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findManyTestModel { composite { field_opt } } }"#),
          @r###"{"data":{"findManyTestModel":[{"composite":{"field_opt":null}}]}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ findUniqueTestModel(where: { id: 1 }) { composite { field_opt } } }"#),
          @r###"{"data":{"findUniqueTestModel":{"composite":{"field_opt":null}}}}"###
        );

        Ok(())
    }

    fn run_command_raw(command: serde_json::Value) -> String {
        let command = command.to_string().replace('\"', "\\\"");

        format!(r#"mutation {{ runCommandRaw(command: "{command}") }}"#)
    }
}

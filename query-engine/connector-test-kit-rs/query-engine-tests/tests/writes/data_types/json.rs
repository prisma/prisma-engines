use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema))]
mod json {
    fn schema() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, String, @id)
              field Json?  @default("{}")
             }"#
        };

        schema.to_owned()
    }

    // "Json float accuracy" should "work"
    #[connector_test(exclude(SqlServer, Mysql, Sqlite))]
    async fn json_float_accuracy(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModel(
              data: {
                id: "B"
                field: "0.9215686321258545"
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"createOneModel":{"field":"0.9215686321258545"}}}"###
        );

        Ok(())
    }

    // "Using a json field" should "work"
    #[connector_test]
    async fn using_json_field(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            createOneModel(
              data: {
                id: "A"
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"createOneModel":{"field":"{}"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneModel(
              where: { id: "A" }
              data: {
                field: "{\\"a\\":\\"b\\"}"
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"updateOneModel":{"field":"{\"a\":\"b\"}"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation {
            updateOneModel(
              where: { id: "A" }
              data: {
                field: null
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"updateOneModel":{"field":null}}}"###
        );

        Ok(())
    }
}

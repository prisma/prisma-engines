use query_engine_tests::*;

#[test_suite(capabilities(Json), schema(schema))]
mod json {
    use indoc::indoc;
    use query_engine_tests::run_query;

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
    #[connector_test(exclude(SqlServer, MySQL, Vitess, Sqlite))]
    async fn json_float_accuracy(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
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
}

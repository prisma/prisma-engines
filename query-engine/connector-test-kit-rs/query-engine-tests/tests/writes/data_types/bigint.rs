use query_engine_tests::*;

#[test_suite(schema(schema))]
mod bigint {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Model {
              #id(id, Int, @id)
              field BigInt? @default(123456789012341234)
             }"#
        };

        schema.to_owned()
    }

    // "Using a BigInt field" should "work"
    #[connector_test()]
    async fn using_bigint_field(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneModel(
              data: {
                id: 1
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"createOneModel":{"field":"123456789012341234"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModel(
              where: { id: 1 }
              data: {
                field: "9223372036854775807"
              }
            ) {
              field
            }
          }"#),
          @r###"{"data":{"updateOneModel":{"field":"9223372036854775807"}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            updateOneModel(
              where: { id: 1 }
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

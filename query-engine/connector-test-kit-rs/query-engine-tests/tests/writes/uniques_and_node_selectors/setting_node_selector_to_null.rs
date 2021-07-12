use query_engine_tests::*;

#[test_suite(schema(schema))]
mod node_sel_to_null {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model A {
              #id(id, Int, @id)
              b   String? @unique
              key String  @unique
            }"#
        };

        schema.to_owned()
    }

    // "Setting a where value to null " should " work when there is no further nesting "
    #[connector_test]
    async fn where_val_to_null(runner: &Runner) -> TestResult<()> {
        run_query!(
            runner,
            r#"mutation {
                createOneA(data: {
                  id: 1,
                  b: "abc"
                  key: "abc"
                }) {
                  id
                }
            }"#
        );

        insta::assert_snapshot!(
          run_query!(runner, r#"mutation b {
            updateOneA(
              where: { b: "abc" }
              data: {
                b: { set: null }
              }) {
              b
            }
          }"#),
          @r###"{"data":{"updateOneA":{"b":null}}}"###
        );

        Ok(())
    }
}

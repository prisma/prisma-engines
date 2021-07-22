use query_engine_tests::*;

#[test_suite(schema(schema))]
mod non_uniq_index {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              field String?

              @@index([field], name: "test_index")
            }"#
        };

        schema.to_owned()
    }

    // "Non-unique indices" should "not cause unique filters for that field to show up"
    #[connector_test]
    async fn non_uniq_indices(runner: Runner) -> TestResult<()> {
        run_query!(
            &runner,
            r#"mutation {
          createOneTestModel(data: { id: 1, field: "Test" }) {
              id
            }
          }"#
        );

        assert_error!(
            &runner,
            r#"query {
              findUniqueTestModel(where: { field: "nope" }) {
                  id
              }
            }"#,
            2009
        );

        Ok(())
    }
}

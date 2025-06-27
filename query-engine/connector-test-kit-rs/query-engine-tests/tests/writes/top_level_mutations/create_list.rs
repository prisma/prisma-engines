use query_engine_tests::*;

#[test_suite(schema(schema), capabilities(ScalarLists))]
mod create_list {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model User {
              #id(id, Int, @id)
              test    String[]
            }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation" should "should not accept null in set"
    #[connector_test]
    async fn create_not_accept_null_in_set(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {createOneUser(data: { id: 1, test: {set: null} }) { id, test }}"#,
            2009
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneUser(data: { id: 1 }){ id, test }}"#),
          @r###"{"data":{"createOneUser":{"id":1,"test":null}}}"###
        );

        Ok(())
    }
}

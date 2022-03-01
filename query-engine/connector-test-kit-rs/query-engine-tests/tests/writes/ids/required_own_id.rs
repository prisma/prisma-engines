use query_engine_tests::*;

#[test_suite(schema(schema))]
mod required_own_id {
    use indoc::indoc;
    use query_engine_tests::{assert_error, run_query};

    fn schema() -> String {
        let schema = indoc! {
            r#"model ScalarModel {
              #id(id, String, @id)
              optString   String?
           }"#
        };

        schema.to_owned()
    }

    // "A Create Mutation" should "create and return item"
    #[connector_test]
    async fn create_mut_return_item(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation {
            createOneScalarModel(data: {
              id: "thisismyownid"
            }){ id }
          }"#),
          @r###"{"data":{"createOneScalarModel":{"id":"thisismyownid"}}}"###
        );

        Ok(())
    }

    // "A Create Mutation" should "error if a required id is not provided"
    #[connector_test]
    async fn error_if_required_id_not_provided(runner: Runner) -> TestResult<()> {
        assert_error!(
            &runner,
            r#"mutation {
                createOneScalarModel(data: {
                  optString: "iforgotmyid"
                }){ id }
            }"#,
            2009
        );
        Ok(())
    }
}

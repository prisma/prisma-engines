use query_engine_tests::*;

#[test_suite]
mod schema_gen {
    use query_engine_tests::{connector_test, run_query, Runner};
    use query_test_macros::connector_schema_gen;

    #[connector_schema_gen(gen(ParentList, ChildList, without_params = true))]
    async fn schema_gen(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#"query { findManyTestModel { id field } }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }
}

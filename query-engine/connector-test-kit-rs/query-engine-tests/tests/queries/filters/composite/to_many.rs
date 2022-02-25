use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(to_many_composites))]
mod element {
    #[connector_test]
    async fn vanilla(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(runner, r#""#),
          @r###""###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        // a few full data
        create_row(runner, r#"{ as: }"#).await?;

        // a few with empty list

        // a few with no list

        create_row(runner, r#"{ uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?;

        Ok(())
    }
}

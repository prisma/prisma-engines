use query_engine_tests::*;

// Validates fix for: "Incorrect handling of "undefined" in queries"
// https://github.com/prisma/prisma/issues/4088

#[test_suite(schema(schema))]
mod prisma_4088 {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"model TestModel {
              #id(id, Int, @id)
              str String
            }"#
        };

        schema.to_owned()
    }

    // "FindMany queries with an OR condition and one filter" should "only apply one filter"
    #[connector_test]
    async fn find_many_or_cond_one_filter(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { OR: [{ str: { equals: "aa" } }]}
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"aa"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an OR condition and two filters, of which one is undefined" should "only apply one filter"
    #[connector_test]
    async fn find_many_or_cond_two_filters(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { OR: [{ str: { equals: "aa" }}, {str: {} }]}
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"aa"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an OR condition and no filters" should "return an empty list"
    #[connector_test]
    async fn find_many_or_cond_no_filter(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { OR: [] }
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an AND condition and no filters" should "return all items"
    #[connector_test]
    async fn find_many_and_cond_no_filter(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { AND: [] }
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"aa"},{"str":"ab"},{"str":"ac"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an AND condition and one filter" should "only apply one filter"
    #[connector_test]
    async fn find_many_and_cond_one_filter(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { AND: [{ str: { equals: "aa" } }]}
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"aa"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an AND condition and two filters, of which one is undefined" should "only apply one filter"
    #[connector_test]
    async fn find_many_and_cond_two_filters(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { AND: [{ str: { equals: "aa" }}, {str: {} }]}
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"aa"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an NOT condition and no filters" should "return all items"
    #[connector_test]
    async fn find_many_not_cond_no_filter(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { NOT: [] }
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"aa"},{"str":"ab"},{"str":"ac"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an NOT condition and one filter " should "only apply one filter"
    #[connector_test]
    async fn find_many_not_cond_one_filter(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { NOT: [{ str: { equals: "aa" } }] }
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"ab"},{"str":"ac"}]}}"###
        );

        Ok(())
    }

    // "FindMany queries with an NOT condition and two filters, of which one is undefined" should "only apply one filter"
    #[connector_test]
    async fn find_many_not_cond_two_filters(runner: &Runner) -> TestResult<()> {
        create_test_data(runner).await?;

        insta::assert_snapshot!(
          run_query!(runner, r#"query {
            findManyTestModel(
              where: { NOT: [{ str: { equals: "aa" }}, {str: {} }]}
            ) {
              str
            }
          }"#),
          @r###"{"data":{"findManyTestModel":[{"str":"ab"},{"str":"ac"}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, str: "aa" }"#).await?;
        create_row(runner, r#"{ id: 2, str: "ab" }"#).await?;
        create_row(runner, r#"{ id: 3, str: "ac" }"#).await?;

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();
        Ok(())
    }
}

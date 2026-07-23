use query_engine_tests::*;

#[test_suite(schema(schema))]
mod sr_regression {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Category {
                #id(id, String, @id, @default(cuid()))
                name      String
                parent_id String? @unique

                parent   Category? @relation(name: "C", fields: [parent_id], references: [id], onDelete: NoAction, onUpdate: NoAction)
                opposite Category? @relation(name: "C")
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn all_categories(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCategory(orderBy: { name: asc }) { name parent { name }}}"#),
          @r###"{"data":{"findManyCategory":[{"name":"Root","parent":null},{"name":"Sub","parent":{"name":"Root"}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn root_categories(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCategory(where: { parent: { is: null }}) { name parent { name }}}"#),
          @r###"{"data":{"findManyCategory":[{"name":"Root","parent":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn inverted_subcat(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCategory(where: { NOT: [{ parent: { is: null }}] }) { name parent { name }}}"#),
          @r###"{"data":{"findManyCategory":[{"name":"Sub","parent":{"name":"Root"}}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn subcat_scalar(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyCategory(where: { parent: { is: { name: { equals: "Root" }}}}) { name parent { name }}}"#),
          @r###"{"data":{"findManyCategory":[{"name":"Sub","parent":{"name":"Root"}}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneCategory(data: { name: "Sub", parent: { create: { name: "Root" }}}) { parent { id }}}"#)
            .await?.assert_success();

        Ok(())
    }
}

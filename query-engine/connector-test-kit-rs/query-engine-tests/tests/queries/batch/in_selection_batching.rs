use query_engine_tests::*;

/// Port note: Batch size for testing is now 10 by default, not configurable (look at the direnv).
#[test_suite(schema(schema))]
mod isb {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model A {
                #id(id, Int, @id)
                b_id Int
                c_id Int

                b B @relation(fields: [b_id], references: [id])
                c C @relation(fields: [c_id], references: [id])
              }

              model B {
                #id(id, Int, @id)
                as A[]
              }

              model C {
                #id(id, Int, @id)
                as A[]
              }
            "#
        };

        schema.to_owned()
    }

    // "batching of IN queries" should "work when having more than the specified amount of items"
    #[connector_test]
    async fn in_more_items(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: { id: { in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // "ascending ordering of batched IN queries" should "work when having more than the specified amount of items"
    #[connector_test]
    async fn asc_in_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: { id: { in: [5,4,3,2,1,2,1,1,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}, orderBy: { id: asc }) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":1},{"id":2},{"id":3},{"id":4},{"id":5}]}}"###
        );

        Ok(())
    }

    // "ascending ordering of batched IN queries" should "work when having more than the specified amount of items"
    #[connector_test]
    async fn desc_in_ordering(runner: Runner) -> TestResult<()> {
        create_test_data(&runner).await?;

        insta::assert_snapshot!(
          run_query!(&runner, indoc! { r#"
            query {
              findManyA(where: {id: { in: [5,4,3,2,1,1,1,2,3,4,5,6,7,6,5,4,3,2,1,2,3,4,5,6] }}, orderBy: { id: desc }) { id }
            }"# }),
          @r###"{"data":{"findManyA":[{"id":5},{"id":4},{"id":3},{"id":2},{"id":1}]}}"###
        );

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_a(runner, r#"{ id: 1, b: { create: { id: 1 }} c: { create: { id: 1 }} }"#).await?;
        create_a(runner, r#"{ id: 2, b: { connect: { id: 1 }} c: { create: { id: 2 }} }"#).await?;
        create_a(runner, r#"{ id: 3, b: { create: { id: 3 }} c: { create: { id: 3 }} }"#).await?;
        create_a(runner, r#"{ id: 4, b: { create: { id: 4 }} c: { create: { id: 4 }} }"#).await?;
        create_a(runner, r#"{ id: 5, b: { create: { id: 5 }} c: { create: { id: 5 }} }"#).await?;

        Ok(())
    }

    async fn create_a(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneA(data: {}) {{ id }} }}", data))
            .await?
            .assert_success();

        Ok(())
    }
}

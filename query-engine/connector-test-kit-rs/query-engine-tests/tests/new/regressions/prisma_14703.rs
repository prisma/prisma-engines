use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), exclude(MongoDB, Sqlite))]
mod prisma_14703 {
    fn schema() -> String {
        String::from(indoc! {r#"
            model A {
              id Decimal @id @test.Decimal(10, 0)
            }
        "#})
    }

    #[connector_test]
    async fn upsert_does_not_panic_if_underflowing_the_scale(runner: Runner) -> TestResult<()> {
        let query = indoc! {r#"
            mutation {
              createOneA(data: { id: "90" }) { id }
            }
        "#};

        let response = runner.query(query).await?;
        response.assert_success();

        let query = indoc! {r#"
            mutation {
              upsertOneA(where: { id: "90" }, create: { id: "90" }, update: { id: "900" } ) { id }
            }
        "#};

        let response = runner.query(query).await?;
        response.assert_success();

        Ok(())
    }
}

use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod mongodb {
    use indoc::indoc;

    fn schema() -> String {
        let schema = indoc! {
            r#"
            model Course {
                id String @id @default(auto()) @map("_id") @test.ObjectId
                rating Float @default(3.5)
            }
            "#
        };
        schema.to_owned()
    }

    #[connector_test]
    async fn gte_works_with_floating_numbers(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneCourse(data: {}) { id } }"#)
            .await?
            .assert_success();

        assert_query!(
            runner,
            r#"query {
            aggregateCourse(where: {
              rating: {
                gte: 3.5
              }
            }) { _count {id} }}"#,
            r#"{"data":{"aggregateCourse":{"_count":{"id":1}}}}"#
        );

        Ok(())
    }

    #[connector_test]
    async fn lte_works_with_floating_numbers(runner: Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneCourse(data: {}) { id } }"#)
            .await?
            .assert_success();

        assert_query!(
            runner,
            r#"query {
            aggregateCourse(where: {
              rating: {
                lte: 3.5
              }
            }) { _count {id} }}"#,
            r#"{"data":{"aggregateCourse":{"_count":{"id":1}}}}"#
        );

        Ok(())
    }
}

use query_engine_tests::*;

/// Regression test for https://github.com/prisma/prisma/issues/6337
#[test_suite(schema(bigint_schema))]
mod bigint_cursor {
    use indoc::indoc;
    use query_engine_tests::run_query;

    fn bigint_schema() -> String {
        let schema = indoc! {"
            model TestModel {
                #id(id, BigInt, @id)
                counter Int
            }
        "};

        schema.to_owned()
    }

    #[connector_test]
    async fn bigint_id_must_work(runner: Runner) -> TestResult<()> {
        test_data(&runner).await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyTestModel(cursor: { id: 2 }, orderBy: { counter: desc }){ id counter }}"),
            @r###"{"data":{"findManyTestModel":[{"id":"2","counter":2},{"id":"1","counter":1}]}}"###
        );

        Ok(())
    }

    async fn test_data(runner: &Runner) -> TestResult<()> {
        runner
            .query(r#"mutation { createOneTestModel(data: { id: 1, counter: 1 }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneTestModel(data: { id: 2, counter: 2 }) { id }}"#)
            .await?
            .assert_success();

        runner
            .query(r#"mutation { createOneTestModel(data: { id: 3, counter: 3 }) { id }}"#)
            .await?
            .assert_success();

        Ok(())
    }
}

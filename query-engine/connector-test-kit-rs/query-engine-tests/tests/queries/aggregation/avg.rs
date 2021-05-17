use query_engine_tests::*;

#[test_suite(schema(schemas::common_numeric_types))]
mod aggregation_avg {
    #[connector_test]
    async fn avg_no_records(runner: &Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { aggregateTestModel { avg { int bInt float } } }"
            ),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":null,"bInt":null,"float":null}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn avg_some_records(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, float: 5.5, int: 5, bInt: "5" }"#).await?;
        create_row(runner, r#"{ id: 2, float: 4.5, int: 10, bInt: "10" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { aggregateTestModel { avg { int bInt float } } }"
            ),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":7.5,"bInt":7.5,"float":5.0}}}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn avg_with_all_sorts_of_query_args(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ id: 1, float: 5.5, int: 5, bInt: "5" }"#).await?;
        create_row(runner, r#"{ id: 2, float: 4.5, int: 10, bInt: "10" }"#).await?;
        create_row(runner, r#"{ id: 3, float: 1.5, int: 2, bInt: "2" }"#).await?;
        create_row(runner, r#"{ id: 4, float: 0.0, int: 1, bInt: "1" }"#).await?;

        insta::assert_snapshot!(
            run_query!(runner, "query { aggregateTestModel(take: 2) { avg { int bInt float } } }"),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":7.5,"bInt":7.5,"float":5.0}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { aggregateTestModel(take: 5) { avg { int bInt float } } }"),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":4.5,"bInt":4.5,"float":2.875}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { aggregateTestModel(take: -5) { avg { int bInt float } } }"),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":4.5,"bInt":4.5,"float":2.875}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"query { aggregateTestModel(where: { id: { gt: 2 }}) { avg { int bInt float } } }"#),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":1.5,"bInt":1.5,"float":0.75}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, "query { aggregateTestModel(skip: 2) { avg { int bInt float } } }"),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":1.5,"bInt":1.5,"float":0.75}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(runner, r#"query { aggregateTestModel(cursor: { id: 3 }) { avg { int bInt float } } }"#),
            @r###"{"data":{"aggregateTestModel":{"avg":{"int":1.5,"bInt":1.5,"float":0.75}}}}"###
        );

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

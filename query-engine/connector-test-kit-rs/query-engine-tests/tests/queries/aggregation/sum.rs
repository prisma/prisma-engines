use query_engine_tests::*;

#[test_suite(schema(schemas::common_numeric_types))]
mod aggregation_sum {
    use query_engine_tests::run_query;

    // TODO: remove exclude once fixed for mongo
    #[connector_test(exclude(MongoDb))]
    async fn sum_no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "query { aggregateTestModel { _sum { int bInt float decimal } } }"),
          @r###"{"data":{"aggregateTestModel":{"_sum":{"int":null,"bInt":null,"float":null,"decimal":null}}}}"###
        );

        Ok(())
    }

    // TODO: remove exclude once fixed for mongo
    #[connector_test(exclude(MongoDb))]
    async fn sum_some_records(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 5.5, int: 5, decimal: "5.5", bInt: "5" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 4.5, int: 10, decimal: "4.5", bInt: "10" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { aggregateTestModel { _sum { int bInt float decimal } } }"
            ),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":15,"bInt":"15","float":10.0,"decimal":"10"}}}}"###
        );

        Ok(())
    }

    // TODO: remove exclude once fixed for mongo
    #[connector_test(exclude(MongoDb))]
    async fn sum_with_all_sorts_of_query_args(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 5.5, int: 5, decimal: "5.5", bInt: "5" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 4.5, int: 10, decimal: "4.5", bInt: "10" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.5, int: 2, decimal: "1.5", bInt: "2" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 0.0, int: 1, decimal: "0.0", bInt: "1" }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 2) { _sum { int bInt float decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":15,"bInt":"15","float":10.0,"decimal":"10"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 5) { _sum { int bInt float decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":18,"bInt":"18","float":11.5,"decimal":"11.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: -5) { _sum { int bInt float decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":18,"bInt":"18","float":11.5,"decimal":"11.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(where: { id: { gt: 2 }}) { _sum { int bInt float decimal } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":3,"bInt":"3","float":1.5,"decimal":"1.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(skip: 2) { _sum { int bInt float decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":3,"bInt":"3","float":1.5,"decimal":"1.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(cursor: { id: 3 }) { _sum { int bInt float decimal } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_sum":{"int":3,"bInt":"3","float":1.5,"decimal":"1.5"}}}}"###
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

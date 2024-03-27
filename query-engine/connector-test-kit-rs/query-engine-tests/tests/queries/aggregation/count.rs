use query_engine_tests::*;

#[test_suite(schema(schemas::common_nullable_types))]
mod aggregation_count {
    use query_engine_tests::run_query;

    #[connector_test]
    async fn count_no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel { _count { _all } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":0}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn count_nullable_fields(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, string: "test1" }"#).await?;
        create_row(&runner, r#"{ id: 2, int: 1 }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel { _count { _all string int } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":2,"string":1,"int":1}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn count_with_all_sorts_of_query_args(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, string: "1" }"#).await?;
        create_row(&runner, r#"{ id: 2, string: "2" }"#).await?;
        create_row(&runner, r#"{ id: 3, string: "3" }"#).await?;
        create_row(&runner, r#"{ id: 4, string: "4" }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 2) { _count { _all } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":2}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 5) { _count { _all } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":4}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: -5) { _count { _all } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":4}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(where: { string: { gt: "2" } }) { _count { _all } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":2}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(where: { string: { gt: "1" }} orderBy: { string: desc }) { _count { _all } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":3}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(skip: 2) { _count { _all } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":2}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(cursor: { id: 2 }) { _count { _all } } }"),
            @r###"{"data":{"aggregateTestModel":{"_count":{"_all":3}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn count_empty_result(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, r#"{ aggregateTestModel { _count { _all string int } } }"#),
          @r###"{"data":{"aggregateTestModel":{"_count":{"_all":0,"string":0,"int":0}}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"{ aggregateTestModel { _count { string _all int } } }"#),
          @r###"{"data":{"aggregateTestModel":{"_count":{"string":0,"_all":0,"int":0}}}}"###
        );

        Ok(())
    }

    async fn create_row(runner: &Runner, data: &str) -> TestResult<()> {
        runner
            .query(format!("mutation {{ createOneTestModel(data: {data}) {{ id }} }}"))
            .await?
            .assert_success();
        Ok(())
    }
}

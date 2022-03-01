use query_engine_tests::*;

#[test_suite(schema(schemas::common_text_and_numeric_types))]
mod aggregation_min {
    use query_engine_tests::run_query;

    #[connector_test]
    async fn min_no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "query { aggregateTestModel { _min { string int bInt float decimal } } }"),
          @r###"{"data":{"aggregateTestModel":{"_min":{"string":null,"int":null,"bInt":null,"float":null,"decimal":null}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn min_some_records(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 5.5, int: 5, decimal: "5.5", bInt: "5", string: "a" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, float: 4.5, int: 10, decimal: "4.5", bInt: "10", string: "b" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { aggregateTestModel { _min { int bInt float decimal string } } }"
            ),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":5,"bInt":"5","float":4.5,"decimal":"4.5","string":"a"}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn min_with_all_sorts_of_query_args(runner: Runner) -> TestResult<()> {
        create_row(
            &runner,
            r#"{ id: 1, float: 5.5, int: 5, decimal: "5.5", bInt: "5", string: "2" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 2, float: 4.5, int: 10, decimal: "4.5", bInt: "10", string: "f" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 3, float: 1.5, int: 2, decimal: "1.5", bInt: "2", string: "z" }"#,
        )
        .await?;
        create_row(
            &runner,
            r#"{ id: 4, float: 0.0, int: 1, decimal: "0.0", bInt: "1", string: "g" }"#,
        )
        .await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 2) { _min { int bInt float decimal string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":5,"bInt":"5","float":4.5,"decimal":"4.5","string":"2"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 5) { _min { int bInt float decimal string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":1,"bInt":"1","float":0.0,"decimal":"0","string":"2"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: -5) { _min { int bInt float decimal string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":1,"bInt":"1","float":0.0,"decimal":"0","string":"2"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(where: { id: { gt: 2 }}) { _min { int bInt float decimal string } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":1,"bInt":"1","float":0.0,"decimal":"0","string":"g"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(skip: 2) { _min { int bInt float decimal string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":1,"bInt":"1","float":0.0,"decimal":"0","string":"g"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(cursor: { id: 3 }) { _min { int bInt float decimal string } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_min":{"int":1,"bInt":"1","float":0.0,"decimal":"0","string":"g"}}}}"###
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

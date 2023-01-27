use query_engine_tests::*;

#[test_suite(schema(schemas::common_text_and_numeric_types))]
mod aggregation_max {
    use query_engine_tests::run_query;

    #[connector_test]
    async fn max_no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "query { aggregateTestModel { _max { string int bInt float } } }"),
          @r###"{"data":{"aggregateTestModel":{"_max":{"string":null,"int":null,"bInt":null,"float":null}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn max_some_records(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 5.5, int: 5, bInt: "5", string: "a" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 4.5, int: 10, bInt: "10", string: "b" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { aggregateTestModel { _max { int bInt float string } } }"
            ),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":10,"bInt":"10","float":5.5,"string":"b"}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn max_with_all_sorts_of_query_args(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, float: 5.5, int: 5, bInt: "5", string: "2" }"#).await?;
        create_row(&runner, r#"{ id: 2, float: 4.5, int: 10, bInt: "10", string: "f" }"#).await?;
        create_row(&runner, r#"{ id: 3, float: 1.5, int: 2, bInt: "2", string: "z" }"#).await?;
        create_row(&runner, r#"{ id: 4, float: 0.0, int: 1, bInt: "1", string: "g" }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 2) { _max { int bInt float string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":10,"bInt":"10","float":5.5,"string":"f"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 5) { _max { int bInt float string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":10,"bInt":"10","float":5.5,"string":"z"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: -5) { _max { int bInt float string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":10,"bInt":"10","float":5.5,"string":"z"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(where: { id: { gt: 2 }}) { _max { int bInt float string } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":2,"bInt":"2","float":1.5,"string":"z"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(skip: 2) { _max { int bInt float string } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":2,"bInt":"2","float":1.5,"string":"z"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(cursor: { id: 3 }) { _max { int bInt float string } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_max":{"int":2,"bInt":"2","float":1.5,"string":"z"}}}}"###
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

#[test_suite(schema(schema), capabilities(DecimalType))]
mod decimal_aggregation_max {
    use query_engine_tests::run_query;

    fn schema() -> String {
        let schema = indoc! {
            "model TestModel {
                #id(id, Int, @id)
                decimal Decimal
            }"
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn max_no_records(runner: Runner) -> TestResult<()> {
        insta::assert_snapshot!(
          run_query!(&runner, "query { aggregateTestModel { _max { decimal } } }"),
          @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":null}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn max_some_records(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, decimal: "5.5" }"#).await?;
        create_row(&runner, r#"{ id: 2, decimal: "4.5" }"#).await?;

        insta::assert_snapshot!(
            run_query!(
                runner,
                "query { aggregateTestModel { _max { decimal } } }"
            ),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"5.5"}}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn max_with_all_sorts_of_query_args(runner: Runner) -> TestResult<()> {
        create_row(&runner, r#"{ id: 1, decimal: "5.5" }"#).await?;
        create_row(&runner, r#"{ id: 2, decimal: "4.5" }"#).await?;
        create_row(&runner, r#"{ id: 3, decimal: "1.5" }"#).await?;
        create_row(&runner, r#"{ id: 4, decimal: "0.0" }"#).await?;

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 2) { _max { decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"5.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: 5) { _max { decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"5.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(take: -5) { _max { decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"5.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(where: { id: { gt: 2 }}) { _max { decimal } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"1.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, "query { aggregateTestModel(skip: 2) { _max { decimal } } }"),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"1.5"}}}}"###
        );

        insta::assert_snapshot!(
            run_query!(&runner, r#"query { aggregateTestModel(cursor: { id: 3 }) { _max { decimal } } }"#),
            @r###"{"data":{"aggregateTestModel":{"_max":{"decimal":"1.5"}}}}"###
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

use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), only(MongoDb))]
mod write_conflict {
    use futures::future;
    use query_engine_tests::Runner;

    fn schema() -> String {
        let schema = indoc! {
            r#"model Invoice {
              #id(id, Int, @id)
              text String?
            }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn conflict_regular(runner: Runner) -> TestResult<()> {
        let futs: Vec<_> = (0..50)
            .map(|i| {
                runner.query(format!(
                    r#"mutation {{ createOneInvoice(data: {{ id: {i} }}) {{ id }} }}"#
                ))
            })
            .collect();

        for res in future::join_all(futs).await {
            res?.assert_success();
        }

        let futs: Vec<_> = (0..50)
            .map(|_| runner.query(r#"mutation { updateManyInvoice(data: { text: "something2" }) { count } }"#))
            .collect();

        for res in future::join_all(futs).await {
            res?.assert_success();
        }

        Ok(())
    }

    async fn create_test_data(runner: &Runner) -> TestResult<()> {
        create_row(runner, r#"{ uniqueField: 1, nonUniqFieldA: "A", nonUniqFieldB: "A"}"#).await?;

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

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
    async fn simple(runner: Runner) -> TestResult<()> {
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

    #[connector_test]
    async fn batched(runner: Runner) -> TestResult<()> {
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

        let queries: Vec<_> = (0..50)
            .map(|_| r#"mutation { updateManyInvoice(data: { text: "something2" }) { count } }"#.to_string())
            .collect();

        let futs: Vec<_> = queries
            .as_slice()
            .windows(10)
            .map(|queries| runner.batch(queries.to_vec(), false, None))
            .collect();

        for res in future::join_all(futs).await {
            res?.assert_success();
        }

        Ok(())
    }
}

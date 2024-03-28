use query_engine_tests::*;

#[test_suite(schema(generic), only(MongoDb))]
mod mongodb {
    use query_engine_tests::query_core::TxId;
    use serde_json::json;
    use std::{future::Future, rc::Rc};

    #[connector_test]
    async fn find_raw_works_with_itx(runner: Runner) -> TestResult<()> {
        run_in_itx(runner, |runner| async move {
            run_query!(
                runner,
                r#"mutation { createOneTestModel(data: { id: 1, field: "A" }) { id }}"#
            );

            insta::assert_snapshot!(
                run_query!(
                    runner,
                    r#"query { findTestModelRaw(filter: "{\"_id\": 1}") }"#
                ),
                @r###"{"data":{"findTestModelRaw":[{"_id":1,"field":"A"}]}}"###
            );

            Ok(())
        })
        .await
    }

    #[connector_test]
    async fn run_command_raw_works_with_itx(runner: Runner) -> TestResult<()> {
        run_in_itx(runner, |runner| async move {
            let command = json!({
                "insert": "TestModel",
                "documents": [
                    { "_id": 1, "field": "A" },
                    { "_id": 2, "field": "B" },
                    { "_id": 3, "field": "C" }
                ]
            });

            insta::assert_snapshot!(
              run_query!(
                runner,
                format!(r#"mutation {{ runCommandRaw(command: "{}") }}"#, command.to_string().replace('\"', "\\\""))
              ),
              @r###"{"data":{"runCommandRaw":{"n":3,"ok":1.0}}}"###
            );

            Ok(())
        })
        .await
    }

    #[connector_test]
    async fn aggregate_raw_works_with_itx(runner: Runner) -> TestResult<()> {
        run_in_itx(runner, |runner| async move {
            run_query!(
                runner,
                r#"mutation { createOneTestModel(data: { id: 1, field: "A" }) { id }}"#
            );

            insta::assert_snapshot!(
                run_query!(
                    runner,
                    r#"query { aggregateTestModelRaw(pipeline: ["{\"$match\": {\"_id\": 1}}"]) }"#
                ),
                @r###"{"data":{"aggregateTestModelRaw":[{"_id":1,"field":"A"}]}}"###
            );

            Ok(())
        })
        .await
    }

    async fn run_in_itx<T, F, O>(mut runner: Runner, f: O) -> TestResult<T>
    where
        F: Future<Output = TestResult<T>>,
        O: FnOnce(Rc<Runner>) -> F,
    {
        let tx_id = start_itx(&mut runner).await?;

        let mut runner = Rc::new(runner);
        let result = f(runner.clone()).await?;
        let runner = Rc::get_mut(&mut runner).unwrap();

        end_itx(runner, tx_id).await?;

        Ok(result)
    }

    async fn start_itx(runner: &mut Runner) -> TestResult<TxId> {
        let tx_id = runner.start_tx(5000, 5000, None, None).await?;
        runner.set_active_tx(tx_id.clone());

        Ok(tx_id)
    }

    async fn end_itx(runner: &mut Runner, tx_id: TxId) -> TestResult<()> {
        let tx_result = runner.commit_tx(tx_id).await?;
        assert!(tx_result.is_ok());

        runner.clear_active_tx();

        Ok(())
    }
}

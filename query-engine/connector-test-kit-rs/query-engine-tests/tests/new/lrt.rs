// use indoc::indoc;
use query_engine_tests::*;

/// LRT = Long-Running Transactions
/// Note that if cache expiration tests fail, make sure `CLOSED_TX_CLEANUP` is set correctly (low value like 2) from the .envrc.
#[test_suite(schema(generic))]
mod lrt {
    use query_core::TransactionError;
    use tokio::time;

    #[connector_test]
    async fn basic_commit_workflow(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.executor().start_tx(5, 5).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "updated" }) { field } }"#),
          @r###"{"data":{"updateOneTestModel":{"field":"updated"}}}"###
        );

        runner.executor().commit_tx(tx_id).await?;
        runner.clear_active_tx();

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":"updated"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn basic_rollback_workflow(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.executor().start_tx(5, 5).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "updated" }) { field } }"#),
          @r###"{"data":{"updateOneTestModel":{"field":"updated"}}}"###
        );

        runner.executor().rollback_tx(tx_id).await?;
        runner.clear_active_tx();

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn tx_expiration_cycle(mut runner: Runner) -> TestResult<()> {
        // Tx expires after one second.
        let tx_id = runner.executor().start_tx(5, 1).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        time::sleep(time::Duration::from_secs(1)).await;
        runner.clear_active_tx();

        // Everything must be rolled back.
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        // Status of the tx must be `Expired`
        let res = runner.executor().commit_tx(tx_id.clone()).await;

        if let Err(query_core::CoreError::TransactionError(txe)) = res {
            assert_eq!(
                txe,
                TransactionError::Closed {
                    reason: "Transaction is no longer valid. Last state: 'Expired'".to_string()
                }
            );
        } else {
            panic!("Expected error, got success.");
        }

        // Wait for cache eviction, no tx should be found.
        time::sleep(time::Duration::from_secs(2)).await;
        let res = runner.executor().commit_tx(tx_id).await;

        if let Err(query_core::CoreError::TransactionError(txe)) = res {
            assert_eq!(txe, TransactionError::NotFound);
        } else {
            panic!("Expected error, got success.");
        }

        Ok(())
    }

    // Timeout / Expiration.
    // No aquisition.
    // Open a tx in a tx.
    // No auto rollback on error.
    // Batches with lrt
    //
}

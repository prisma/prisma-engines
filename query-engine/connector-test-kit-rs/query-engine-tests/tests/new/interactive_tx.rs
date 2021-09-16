use query_engine_tests::test_suite;

/// Note that if cache expiration tests fail, make sure `CLOSED_TX_CLEANUP` is set correctly (low value like 2) from the .envrc.
#[test_suite(schema(generic))]
mod interactive_tx {
    use query_core::TransactionError;
    use query_engine_tests::*;
    use tokio::time;

    #[connector_test]
    async fn basic_commit_workflow(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
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
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
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
        let tx_id = runner.executor().start_tx(5000, 1000).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        time::sleep(time::Duration::from_millis(1500)).await;
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

    #[connector_test]
    async fn no_auto_rollback(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
        runner.set_active_tx(tx_id.clone());

        // Row is created
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // This will error.
        assert_error!(
            &runner,
            r#"mutation { createOneTestModel(data: { doesnt_exist: true }) { id }}"#,
            2009
        );

        // Commit TX, first written row must still be present.
        let res = runner.executor().commit_tx(tx_id.clone()).await;
        assert!(res.is_ok());

        Ok(())
    }

    // Syntax for raw varies too much for a generic test, use postgres for basic testing.
    #[connector_test(only(Postgres))]
    async fn raw_queries(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, fmt_execute_raw("INSERT INTO \"TestModel\"(id, field) VALUES ($1, $2)", vec![PrismaValue::Int(1), PrismaValue::String("Test".to_owned())])),
          @r###"{"data":{"executeRaw":1}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw("SELECT * FROM \"TestModel\"", vec![])),
          @r###"{"data":{"queryRaw":[{"id":1,"field":"Test"}]}}"###
        );

        runner.executor().commit_tx(tx_id.clone()).await?;
        runner.clear_active_tx();

        // Data still there after commit.
        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw("SELECT * FROM \"TestModel\"", vec![])),
          @r###"{"data":{"queryRaw":[{"id":1,"field":"Test"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn batch_queries_success(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
        runner.set_active_tx(tx_id.clone());

        let queries = vec![
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#.to_string(),
        ];

        // Tx flag is not set, but it executes on an ITX.
        runner.batch(queries, false).await?;
        runner.executor().commit_tx(tx_id.clone()).await?;
        runner.clear_active_tx();

        insta::assert_snapshot!(
          run_query!(&runner, "query { findManyTestModel { id }}"),
            @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2},{"id":3}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn batch_queries_rollback(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
        runner.set_active_tx(tx_id.clone());

        let queries = vec![
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#.to_string(),
        ];

        // Tx flag is not set, but it executes on an ITX.
        runner.batch(queries, false).await?;
        runner.executor().rollback_tx(tx_id.clone()).await?;
        runner.clear_active_tx();

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyTestModel { id }}"),
            @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn batch_queries_failure(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.executor().start_tx(5000, 5000).await?;
        runner.set_active_tx(tx_id.clone());

        // One dup key, will cause failure of the batch.
        let queries = vec![
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#.to_string(),
        ];

        // Tx flag is not set, but it executes on an ITX.
        let batch_results = runner.batch(queries, false).await?;
        batch_results.assert_failure(2002, None);

        runner.executor().commit_tx(tx_id.clone()).await?;
        runner.clear_active_tx();

        let partial_data_res = run_query!(&runner, "query { findManyTestModel { id }}");
        match runner.connector() {
            // Postgres and Mongo abort transactions, data is lost.
            ConnectorTag::Postgres(_) | ConnectorTag::MongoDb(_) => insta::assert_snapshot!(
              partial_data_res,
              @r###"{"data":{"findManyTestModel":[]}}"###
            ),
            // Partial data still there because a batch will not be auto-rolled back by other connectors.
            _ => insta::assert_snapshot!(
                partial_data_res,
                @r###"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"###
            ),
        }

        Ok(())
    }

    #[connector_test]
    async fn tx_expiration_failure_cycle(mut runner: Runner) -> TestResult<()> {
        // Tx expires after one seconds.
        let tx_id = runner.executor().start_tx(5000, 1000).await?;
        runner.set_active_tx(tx_id.clone());

        // Row is created
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // This will error.
        assert_error!(
            &runner,
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#,
            2002
        );

        // Wait for tx to expire
        time::sleep(time::Duration::from_millis(1500)).await;

        // Expect the state of the tx to be expired.
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

        Ok(())
    }

    // SQLite fails as it locks the entire table, not allowing the "inner" transaction to finish.
    #[connector_test(exclude(Sqlite))]
    async fn multiple_tx(mut runner: Runner) -> TestResult<()> {
        // First transaction.
        let tx_id_a = runner.executor().start_tx(2000, 2000).await?;

        // Second transaction.
        let tx_id_b = runner.executor().start_tx(2000, 2000).await?;

        // Execute on first transaction.
        runner.set_active_tx(tx_id_a.clone());
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // Switch to second transaction.
        runner.set_active_tx(tx_id_b.clone());
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":2}}}"###
        );

        // Commit second transaction.
        runner.executor().commit_tx(tx_id_b.clone()).await?;

        // Back to first transaction, do a final read and commit.
        runner.set_active_tx(tx_id_a.clone());

        // Mongo for example doesn't read the inner commit value.
        is_one_of!(
            run_query!(&runner, r#"query { findManyTestModel { id }}"#),
            vec![
                r#"{"data":{"findManyTestModel":[{"id":1}]}}"#,
                r#"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"#
            ]
        );

        runner.executor().commit_tx(tx_id_a.clone()).await?;

        Ok(())
    }
}

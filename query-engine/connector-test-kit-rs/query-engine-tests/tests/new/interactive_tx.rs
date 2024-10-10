use query_engine_tests::test_suite;
use std::borrow::Cow;

#[test_suite(schema(generic), exclude(Sqlite("cfd1")))]
mod interactive_tx {
    use std::time::{Duration, Instant};

    use query_engine_tests::*;
    use tokio::time;

    #[connector_test]
    async fn basic_commit_workflow(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "updated" }) { field } }"#),
          @r###"{"data":{"updateOneTestModel":{"field":"updated"}}}"###
        );

        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_ok());
        runner.clear_active_tx();

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":"updated"}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn basic_rollback_workflow(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { updateOneTestModel(where: { id: 1 }, data: { field: "updated" }) { field } }"#),
          @r###"{"data":{"updateOneTestModel":{"field":"updated"}}}"###
        );

        let res = runner.rollback_tx(tx_id).await?;
        assert!(res.is_ok());
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
        let tx_id = runner.start_tx(5000, 1000, None).await?;
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
        let res = runner.commit_tx(tx_id.clone()).await?;

        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();
        println!("KNOWN ERROR {known_err:?}");

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A commit cannot be executed on an expired transaction"));

        // Try again
        let res = runner.commit_tx(tx_id).await?;
        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A commit cannot be executed on an expired transaction"));

        Ok(())
    }

    #[connector_test]
    async fn no_auto_rollback(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.start_tx(5000, 5000, None).await?;
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
        let res = runner.commit_tx(tx_id.clone()).await?;
        assert!(res.is_ok());

        Ok(())
    }

    // Syntax for raw varies too much for a generic test, use postgres for basic testing.
    #[connector_test(only(Postgres))]
    async fn raw_queries(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, fmt_execute_raw("INSERT INTO \"TestModel\"(id, field) VALUES ($1, $2)", vec![RawParam::from(1), RawParam::from("Test")])),
          @r###"{"data":{"executeRaw":1}}"###
        );

        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw("SELECT * FROM \"TestModel\"", vec![])),
          @r###"{"data":{"queryRaw":{"columns":["id","field"],"types":["int","string"],"rows":[[1,"Test"]]}}}"###
        );

        let res = runner.commit_tx(tx_id.clone()).await?;
        assert!(res.is_ok());
        runner.clear_active_tx();

        // Data still there after commit.
        insta::assert_snapshot!(
          run_query!(&runner, fmt_query_raw("SELECT * FROM \"TestModel\"", vec![])),
          @r###"{"data":{"queryRaw":{"columns":["id","field"],"types":["int","string"],"rows":[[1,"Test"]]}}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn batch_queries_success(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        let queries = vec![
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#.to_string(),
        ];

        // Tx flag is not set, but it executes on an ITX.
        runner.batch(queries, false, None).await?;
        let res = runner.commit_tx(tx_id.clone()).await?;
        assert!(res.is_ok());
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
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        let queries = vec![
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#.to_string(),
        ];

        // Tx flag is not set, but it executes on an ITX.
        runner.batch(queries, false, None).await?;
        let res = runner.rollback_tx(tx_id.clone()).await?;
        assert!(res.is_ok());
        runner.clear_active_tx();

        insta::assert_snapshot!(
            run_query!(&runner, "query { findManyTestModel { id }}"),
            @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(Sqlite("cfd1")))]
    async fn batch_queries_failure(mut runner: Runner) -> TestResult<()> {
        // Tx expires after five second.
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        // One dup key, will cause failure of the batch.
        let queries = vec![
            r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 2 }) { id }}"#.to_string(),
            r#"mutation { createOneTestModel(data: { id: 3 }) { id }}"#.to_string(),
        ];

        // Tx flag is not set, but it executes on an ITX.
        let batch_results = runner.batch(queries, false, None).await?;
        batch_results.assert_failure(2002, None);

        let now = Instant::now();
        let res = runner.commit_tx(tx_id.clone()).await?;
        assert!(now.elapsed() <= Duration::from_millis(5000));

        if matches!(runner.connector_version(), ConnectorVersion::MongoDb(_)) {
            assert!(res.is_err());
            let err = res.err().unwrap();
            let known_err = err.as_known().unwrap();
            assert!(known_err.message.contains("has been aborted."));
            assert_eq!(known_err.error_code, "P2028");
        } else {
            assert!(res.is_ok());
        }
        runner.clear_active_tx();

        match_connector_result!(
          &runner,
          "query { findManyTestModel { id }}",
          // Postgres and Mongo abort transactions, data is lost.
          Postgres(_) | MongoDb(_) | CockroachDb(_) => vec![r#"{"data":{"findManyTestModel":[]}}"#],
          // Partial data still there because a batch will not be auto-rolled back by other connectors.
          _ => vec![r#"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"#]
        );

        Ok(())
    }

    #[connector_test]
    async fn tx_expiration_failure_cycle(mut runner: Runner) -> TestResult<()> {
        // Tx expires after one seconds.
        let tx_id = runner.start_tx(5000, 1000, None).await?;
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

        // Expect the state of the tx to be expired so the commit should fail.
        let res = runner.commit_tx(tx_id.clone()).await?;
        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A commit cannot be executed on an expired transaction"));

        // Expect the state of the tx to be expired so the rollback should fail.
        let res = runner.rollback_tx(tx_id.clone()).await?;
        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A rollback cannot be executed on an expired transaction"));

        // Expect the state of the tx to be expired so the query should fail.
        assert_error!(
            runner,
            r#"{ findManyTestModel { id } }"#,
            2028,
            "A query cannot be executed on an expired transaction"
        );

        runner
            .batch(
                vec![
                    "{ findManyTestModel { id } }".to_string(),
                    "{ findManyTestModel { id } }".to_string(),
                ],
                false,
                None,
            )
            .await?
            .assert_failure(
                2028,
                Some("A batch query cannot be executed on an expired transaction".to_string()),
            );

        Ok(())
    }

    // SQLite fails as it locks the entire table, not allowing the "inner" transaction to finish.
    #[connector_test(exclude(Sqlite))]
    async fn multiple_tx(mut runner: Runner) -> TestResult<()> {
        // First transaction.
        let tx_id_a = runner.start_tx(2000, 2000, None).await?;

        // Second transaction.
        let tx_id_b = runner.start_tx(2000, 2000, None).await?;

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
        let res = runner.commit_tx(tx_id_b.clone()).await?;
        assert!(res.is_ok());

        // Back to first transaction, do a final read and commit.
        runner.set_active_tx(tx_id_a.clone());

        // Mongo for example doesn't read the inner commit value.
        is_one_of!(
            run_query!(&runner, r#"query { findManyTestModel { id }}"#),
            [
                r#"{"data":{"findManyTestModel":[{"id":1}]}}"#,
                r#"{"data":{"findManyTestModel":[{"id":1},{"id":2}]}}"#
            ]
        );

        let res = runner.commit_tx(tx_id_a.clone()).await?;

        assert!(res.is_ok());

        Ok(())
    }

    #[connector_test(only(Postgres))]
    async fn write_conflict(mut runner: Runner) -> TestResult<()> {
        // create row
        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1, field: "initial" }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // First transaction.
        let tx_id_a = runner.start_tx(5000, 5000, Some("Serializable".into())).await?;

        // Second transaction.
        let tx_id_b = runner.start_tx(5000, 5000, Some("Serializable".into())).await?;

        // Read on first transaction.
        runner.set_active_tx(tx_id_a.clone());
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":"initial"}]}}"###
        );

        // Read on the second transaction.
        runner.set_active_tx(tx_id_b.clone());
        insta::assert_snapshot!(
            run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
            @r###"{"data":{"findManyTestModel":[{"id":1,"field":"initial"}]}}"###
        );

        // write and commit on the first transaction
        runner.set_active_tx(tx_id_a.clone());
        insta::assert_snapshot!(
            run_query!(&runner, r#"mutation { updateManyTestModel(data: { field: "a" }) { count }}"#),
            @r###"{"data":{"updateManyTestModel":{"count":1}}}"###
        );

        let commit_res = runner.commit_tx(tx_id_a.clone()).await?;
        assert!(commit_res.is_ok());

        // attempt to write on the second transaction
        runner.set_active_tx(tx_id_b.clone());
        let res = runner
            .query(r#"mutation { updateManyTestModel(data: { field: "b" }) { count }}"#)
            .await?;

        res.assert_failure(2034, None);

        Ok(())
    }

    #[connector_test]
    async fn double_commit(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // First commit must be successful
        let res = runner.commit_tx(tx_id.clone()).await?;
        assert!(res.is_ok());

        // Second commit
        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_err());

        runner.clear_active_tx();

        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A commit cannot be executed on a committed transaction"));

        // The first commit must have worked
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":null}]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn double_rollback(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // First rollback must be successful
        let res = runner.rollback_tx(tx_id.clone()).await?;
        assert!(res.is_ok());

        // Second rollback must return error
        let res = runner.rollback_tx(tx_id).await?;
        assert!(res.is_err());

        runner.clear_active_tx();

        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A rollback cannot be executed on a transaction that was rolled back"));

        // Check that the rollback still worked
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn commit_after_rollback(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // Rollback must be successful
        let res = runner.rollback_tx(tx_id.clone()).await?;
        assert!(res.is_ok());

        // Commit must fail
        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_err());

        runner.clear_active_tx();

        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A commit cannot be executed on a transaction that was rolled back"));

        // Check that the commit didn't work
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[]}}"###
        );

        Ok(())
    }

    #[connector_test]
    async fn rollback_after_commit(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, None).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        // Commit must be successful
        let res = runner.commit_tx(tx_id.clone()).await?;
        assert!(res.is_ok());

        // Rollback must fail
        let res = runner.rollback_tx(tx_id).await?;
        assert!(res.is_err());

        runner.clear_active_tx();

        let error = res.err().unwrap();
        let known_err = error.as_known().unwrap();

        assert_eq!(known_err.error_code, Cow::Borrowed("P2028"));
        assert!(known_err
            .message
            .contains("A rollback cannot be executed on a committed transaction"));

        // Check that the commit worked
        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":null}]}}"###
        );

        Ok(())
    }
}

#[test_suite(schema(generic), exclude(Sqlite("cfd1")))]
mod itx_isolation {
    use query_engine_tests::*;

    // All (SQL) connectors support serializable.
    #[connector_test(exclude(MongoDb, Sqlite("cfd1")))]
    async fn basic_serializable(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, Some("Serializable".to_owned())).await?;
        runner.set_active_tx(tx_id.clone());

        insta::assert_snapshot!(
          run_query!(&runner, r#"mutation { createOneTestModel(data: { id: 1 }) { id }}"#),
          @r###"{"data":{"createOneTestModel":{"id":1}}}"###
        );

        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_ok());
        runner.clear_active_tx();

        insta::assert_snapshot!(
          run_query!(&runner, r#"query { findManyTestModel { id field }}"#),
          @r###"{"data":{"findManyTestModel":[{"id":1,"field":null}]}}"###
        );

        Ok(())
    }

    #[connector_test(exclude(MongoDb, Sqlite("cfd1")))]
    async fn casing_doesnt_matter(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, Some("sErIaLiZaBlE".to_owned())).await?;
        runner.set_active_tx(tx_id.clone());

        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_ok());

        Ok(())
    }

    #[connector_test(only(Postgres))]
    async fn spacing_doesnt_matter(mut runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, Some("Repeatable Read".to_owned())).await?;
        runner.set_active_tx(tx_id.clone());

        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_ok());

        let tx_id = runner.start_tx(5000, 5000, Some("RepeatableRead".to_owned())).await?;
        runner.set_active_tx(tx_id.clone());

        let res = runner.commit_tx(tx_id).await?;
        assert!(res.is_ok());

        Ok(())
    }

    #[connector_test(exclude(MongoDb))]
    async fn invalid_isolation(runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, Some("test".to_owned())).await;

        match tx_id {
            Ok(_) => panic!("Expected invalid isolation level string to throw an error, but it succeeded instead."),
            Err(err) => assert!(err.to_string().contains("Invalid isolation level `test`")),
        };

        Ok(())
    }

    // Mongo doesn't support isolation levels.
    #[connector_test(only(MongoDb))]
    async fn mongo_failure(runner: Runner) -> TestResult<()> {
        let tx_id = runner.start_tx(5000, 5000, Some("Serializable".to_owned())).await;

        match tx_id {
            Ok(_) => panic!("Expected mongo to throw an unsupported error, but it succeeded instead."),
            Err(err) => assert!(err.to_string().contains(
                "Unsupported connector feature: Mongo does not support setting transaction isolation levels"
            )),
        };

        Ok(())
    }
}

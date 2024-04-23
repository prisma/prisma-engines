use query_engine_tests::*;

#[test_suite(schema(generic), only(Sqlite("libsql.js")))]
mod sqlite {
    #[connector_test]
    async fn close_tx_on_error(runner: Runner) -> TestResult<()> {
        // Try to open a transaction with unsupported isolation error in SQLite.
        let result = runner.start_tx(2000, 5000, Some("ReadUncommitted".to_owned())).await;
        assert!(result.is_err());

        // Without the changes from https://github.com/prisma/prisma-engines/pull/4286 or
        // https://github.com/prisma/prisma-engines/pull/4489 this second `start_tx` call will
        // either hang infinitely with libSQL driver adapter, or fail with a "cannot start a
        // transaction within a transaction" error.
        // A more future proof way to check this would be to make both transactions EXCLUSIVE or
        // IMMEDIATE if we had control over SQLite transaction type here, as that would not rely on
        // both transactions using the same connection if we were to pool multiple SQLite
        // connections in the future.
        let tx = runner.start_tx(2000, 5000, None).await?;
        runner.rollback_tx(tx).await?.unwrap();

        Ok(())
    }
}

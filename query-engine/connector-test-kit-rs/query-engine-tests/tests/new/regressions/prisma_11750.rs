use query_engine_tests::*;

/// Regression test for <https://github.com/prisma/prisma/issues/11750>.
///
/// See also <https://github.com/prisma/prisma/pull/12563> and
/// <https://github.com/prisma/prisma-engines/pull/2811>.
///
/// This is a port of the TypeScript test from the client test suite.
///
/// The test creates a user and then tries to update the same row in multiple concurrent
/// transactions. We don't assert that most operations succeed and merely log the errors happening
/// during update or commit, as those are expected to happen. We do fail the test if creating the
/// user fails, or if we fail to start a transaction, as those operations are expected to succeed.
///
/// What we really test here, though, is that the query engine must not deadlock (leading to the
/// test never finishing).
///
/// Some providers are skipped because these concurrent conflicting transactions cause troubles on
/// the database side and failures to start new transactions.
/// TODO: investigate the problem in pg and neon JS driver adapters, looks like some error is not
/// being handled properly in them.
///
/// For an example of an equivalent test that passes on all databases where the transactions don't
/// conflict and don't cause issues on the database side, see the `high_concurrency` test in the
/// `new::interactive_tx::interactive_tx` test suite.
#[test_suite(
    schema(user),
    exclude(Sqlite, MySql(8), SqlServer, Postgres("pg.js"), Postgres("neon.js"))
)]
mod prisma_11750 {
    use std::sync::Arc;
    use tokio::task::JoinSet;

    #[connector_test]
    async fn test_itx_concurrent_updates_single_thread(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        create_user(&runner, 1, "x").await?;

        for _ in 0..10 {
            tokio::try_join!(
                update_user(Arc::clone(&runner), "a"),
                update_user(Arc::clone(&runner), "b"),
                update_user(Arc::clone(&runner), "c"),
                update_user(Arc::clone(&runner), "d"),
                update_user(Arc::clone(&runner), "e"),
                update_user(Arc::clone(&runner), "f"),
                update_user(Arc::clone(&runner), "g"),
                update_user(Arc::clone(&runner), "h"),
                update_user(Arc::clone(&runner), "i"),
                update_user(Arc::clone(&runner), "j"),
            )?;
        }

        create_user(&runner, 2, "y").await?;

        Ok(())
    }

    #[connector_test]
    async fn test_itx_concurrent_updates_multi_thread(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        create_user(&runner, 1, "x").await?;

        for _ in 0..10 {
            let mut set = JoinSet::new();

            for email in ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"] {
                set.spawn(update_user(Arc::clone(&runner), email));
            }

            while let Some(handle) = set.join_next().await {
                handle.expect("task panicked or canceled")?;
            }
        }

        create_user(&runner, 2, "y").await?;

        Ok(())
    }

    async fn create_user(runner: &Runner, id: u32, email: &str) -> TestResult<()> {
        run_query!(
            &runner,
            format!(
                r#"mutation {{
                    createOneUser(
                        data: {{
                            id: {id},
                            first_name: "{email}",
                            last_name: "{email}",
                            email: "{email}"
                        }}
                    ) {{ id }}
                }}"#
            )
        );

        Ok(())
    }

    async fn update_user(runner: Arc<Runner>, new_email: &str) -> TestResult<()> {
        let tx_id = runner.start_tx(2000, 25, None).await?;

        let result = runner
            .query_in_tx(
                &tx_id,
                format!(
                    r#"mutation {{
                        updateOneUser(
                            where: {{ id: 1 }},
                            data: {{ email: "{new_email}" }}
                        ) {{ id }}
                    }}"#
                ),
            )
            .await;

        if let Err(err) = result {
            tracing::error!(%err, "query error");
        }

        let result = runner.commit_tx(tx_id).await?;

        if let Err(err) = result {
            tracing::error!(?err, "commit error");
        }

        Ok(())
    }
}

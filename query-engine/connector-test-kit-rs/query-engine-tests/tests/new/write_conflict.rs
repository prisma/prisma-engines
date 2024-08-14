use indoc::formatdoc;
use indoc::indoc;
use query_engine_tests::*;

#[test_suite(schema(schema), exclude(MongoDb))]
mod write_conflict {
    use query_engine_tests::Runner;
    use std::sync::Arc;
    use tokio::sync::{Mutex, MutexGuard};

    fn schema() -> String {
        let schema = indoc! {
            r#"
              model User {
                id      String   @id
                email   String   @unique
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    async fn concurrent_create_select(runner: Runner) -> TestResult<()> {
        let n_users_per_transaction = 5;
        let n_concurrent_transactions = 20;

        let runner = Arc::new(Mutex::new(runner));

        async fn transaction_task(runner: Arc<Mutex<Runner>>, user_ids: Vec<String>) -> TestResult<(usize, usize)> {
            let mut runner_guard = runner.lock().await;
            let tx_id = runner_guard
                .start_tx(5000, 5000, Some("SERIALIZABLE".to_owned()))
                .await?;
            runner_guard.set_active_tx(tx_id.clone());
            drop(runner_guard);

            // Count users before insertion
            let runner_guard = runner.lock().await;
            let count_before = run_count_users(&runner_guard).await?;
            drop(runner_guard);

            // Insert users sequentially
            for user_id in user_ids {
                let runner_guard = runner.lock().await;
                run_create_user(&runner_guard, &user_id).await?;
            }

            // Count users after insertion
            let runner_guard = runner.lock().await;
            let count_after = run_count_users(&runner_guard).await?;
            drop(runner_guard);

            let mut runner_guard = runner.lock().await;
            let res = runner_guard.commit_tx(tx_id).await?;
            assert!(res.is_ok());
            runner_guard.clear_active_tx();
            drop(runner_guard);

            Ok((count_before, count_after))
        }

        let transaction_tasks: Vec<_> = (0..n_concurrent_transactions)
            .map(|i| {
                let runner = Arc::clone(&runner);
                let user_ids = (1..=n_users_per_transaction)
                    .map(|j| format!("u{:05}", i * n_users_per_transaction + j))
                    .collect::<Vec<String>>();

                tokio::spawn(async move { transaction_task(runner, user_ids).await })
            })
            .collect();

        let results = futures::future::join_all(transaction_tasks).await;

        // Process results
        let counts: Vec<(usize, usize)> = results
            .into_iter()
            .map(|r| r.expect("Task panicked").expect("Transaction failed"))
            .collect();

        // Verify results
        assert_eq!(counts.len(), n_concurrent_transactions);
        for (before, after) in counts {
            assert_eq!(after - before, n_users_per_transaction);
        }

        Ok(())
    }

    async fn run_count_users(runner: &MutexGuard<'_, Runner>) -> TestResult<usize> {
        let res = run_query_json!(&runner, "query { findManyUser { id }}");

        let records = res["data"]["findManyUser"]
            .as_array()
            .unwrap()
            .iter()
            .map(|user| user["id"].as_str().unwrap())
            .collect::<Vec<&str>>();

        // dbg!("Found users: {:?}", &records);
        Ok(records.len())
    }

    async fn run_create_user(runner: &MutexGuard<'_, Runner>, id: &str) -> TestResult<String> {
        Ok(run_query!(
            runner,
            formatdoc! { r#"
            mutation {{
                createOneUser(data: {{ id: "{id}", email: "{id}@test.com" }}) {{
                    id
                    email
                }}
            }}
            "#
            }
        ))
    }
}

#[test_suite(schema(schema), only(MongoDb))]
mod write_conflict_mongo {
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

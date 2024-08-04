use indoc::formatdoc;
use query_engine_tests::*;
use std::sync::Arc;

#[test_suite(schema(schema), only(Sqlite))]
mod prisma_concurrent_write {
    fn schema() -> String {
        let schema = indoc! {
            r#"
              model User {
                id      String   @id
                email   String   @unique
                profile Profile?
              }

              model Profile {
                id     String @id @default(uuid())
                user   User   @relation(fields: [userId], references: [id])
                userId String @unique
              }
            "#
        };

        schema.to_owned()
    }

    #[connector_test]
    // Runs 100 `run_create_user` queries in parallel, followed by 100 `run_create_profile` queries in parallel.
    async fn concurrent_creates_should_succeed(runner: Runner) -> TestResult<()> {
        let n = 100;
        let ids: Vec<String> = (1..=n).map(|i| format!("{:05}", i)).collect();

        let runner_arc = Arc::new(runner);

        let create_user_tasks: Vec<_> = ids
            .iter()
            .map(|id| {
                let runner = runner_arc.clone();
                let id = id.clone();
                tokio::spawn(async move { run_create_user(runner, &id).await })
            })
            .collect();

        let created_users: Vec<TestResult<String>> = futures::future::join_all(create_user_tasks)
            .await
            .into_iter()
            .map(|res| res.expect("Task panicked"))
            .collect();

        assert_eq!(created_users.len(), n);

        let create_profile_tasks: Vec<_> = ids
            .iter()
            .map(|id| {
                let runner = runner_arc.clone();
                let id = id.clone();
                tokio::spawn(async move { run_create_profile(runner, &id).await })
            })
            .collect();

        let queries: Vec<TestResult<String>> = futures::future::join_all(create_profile_tasks)
            .await
            .into_iter()
            .map(|res| res.expect("Task panicked"))
            .collect();

        assert_eq!(queries.len(), n);

        Ok(())
    }

    #[connector_test]
    // Runs 2 `run_create_user` queries in parallel, followed by 2 `run_upsert_profile` queries in parallel.
    async fn concurrent_upserts_should_succeed(runner: Runner) -> TestResult<()> {
        let n = 2;
        let ids: Vec<String> = (1..=n).map(|i| format!("{:05}", i)).collect();

        let runner_arc = Arc::new(runner);

        let create_user_tasks: Vec<_> = ids
            .iter()
            .map(|id| {
                let runner = runner_arc.clone();
                let id = id.clone();
                tokio::spawn(async move { run_create_user(runner, &id).await })
            })
            .collect();

        // Collect the results from the spawned tasks
        let created_users: Vec<TestResult<String>> = futures::future::join_all(create_user_tasks)
            .await
            .into_iter()
            .map(|res| res.expect("Task panicked"))
            .collect();

        assert_eq!(created_users.len(), n);

        let upsert_profile_tasks: Vec<_> = ids
            .iter()
            .map(|id| {
                let runner = runner_arc.clone();
                let id = id.clone();
                tokio::spawn(async move { run_upsert_profile(runner, &id).await })
            })
            .collect();

        // Collect the results from the spawned tasks
        let queries: Vec<TestResult<String>> = futures::future::join_all(upsert_profile_tasks)
            .await
            .into_iter()
            .map(|res| res.expect("Task panicked"))
            .collect();

        assert_eq!(queries.len(), n);

        Ok(())
    }

    async fn run_create_user(runner: Arc<Runner>, id: &str) -> TestResult<String> {
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

    async fn run_create_profile(runner: Arc<Runner>, id: &str) -> TestResult<String> {
        Ok(run_query!(
            runner,
            formatdoc! { r#"
            mutation {{
                createOneProfile(
                    data: {{
                        user: {{
                            connect: {{ id: "{id}" }}
                        }}
                    }}
                ) {{
                    id
                }}
            }}
            "# }
        ))
    }

    async fn run_upsert_profile(runner: Arc<Runner>, id: &str) -> TestResult<String> {
        Ok(run_query!(
            runner,
            formatdoc! { r#"
            mutation {{
                upsertOneProfile(where: {{
                    id: "{id}"
                }}, create: {{
                    user: {{
                        connect: {{ id: "{id}" }}
                    }}
                }}, update: {{
                }}) {{
                    id
                }}
            }}
          "# }
        ))
    }
}

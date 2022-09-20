use query_engine_tests::test_suite;

#[test_suite]
mod upsert_retry {
    use std::sync::Arc;

    use query_engine_tests::*;

    pub fn upsert_schema() -> String {
        let schema = indoc! {
            r#"
            model Post {
                #id(id, Int, @id)
                title     String
                content   String?
                author    User     @relation(fields: [authorId], references: [id])
                authorId  Int
              }
              
              model User {
                #id(id, Int, @id)
                email   String   @unique
                name    String?
                posts   Post[]
              }
            
            "#
        };

        schema.to_owned()
    }

    #[connector_test(schema(upsert_schema))]
    async fn upsert_retry_should_work(runner: Runner) -> TestResult<()> {
        let runner = Arc::new(runner);

        let mut set = tokio::task::JoinSet::new();

        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));
        set.spawn(run_upsert(runner.clone()));

        let mut updated = 0;
        let mut created = 0;
        while let Some(res) = set.join_next().await {
            let name_update = res.unwrap();

            if name_update == "update".to_string() {
                updated += 1;
            } else if name_update == "create".to_string() {
                created += 1;
            }
        }

        assert_eq!(created, 1);
        assert_eq!(updated, 7);
        Ok(())
    }

    async fn run_upsert(runner: Arc<Runner>) -> String {
        let upsert_user = r#"
            mutation {
                upsertOneUser(where: {email: "user-email@test.com"},
                create: {
                    name: "create",
                    id: 1,
                    email: "user-email@test.com",
                    posts: {
                        createMany: { data: [{ id: 1, title: "post1" }, { id: 2, title: "post2" }] },
                    }
                }, update: {
                    name: "update"
                }    
            ) {
                id,
                name
            }
        }
        "#;
        let res = runner.query(upsert_user).await.unwrap().to_json_value();
        res["data"]["upsertOneUser"]["name"].as_str().unwrap().to_string()
    }
}

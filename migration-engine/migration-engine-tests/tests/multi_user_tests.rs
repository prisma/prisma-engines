use migration_engine_tests::{sql::multi_user::*, TestResult};

#[test_each_connector]
async fn multi_users_sanity_check(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id String @id
            name String
            age Int
        }
    "#;

    // Create Alice.
    let alice = {
        let alice = api.new_user("alice", dm).await?;
        assert_eq!(alice.schema_string()?, dm);

        alice
    };

    // Create Bob, starting by cloning Alice's repo.
    let bob = {
        let bob = api.user_cloned_from(&alice, "bob").await?;

        assert_eq!(bob.schema_string()?, dm);

        bob
    };

    // Alice creates her first migration.
    {
        alice.save("init").execute().await?;

        assert_eq!(alice.migrations_in_folder()?, &["init"]);
        assert_eq!(bob.migrations_in_folder()?, &[] as &[&str]);
    };

    // Bob changes the schema and creates his own initial migration
    {
        let dm2 = r#"
            model Cat {
                id String @id
                name String
            }

            model Dog {
                id String @id
                isGoodDog Boolean @default(true)
            }
        "#;

        bob.write_schema(dm2)?;
        assert_eq!(bob.schema_string()?, dm2);

        bob.save("add_dogs").execute().await?;
        assert_eq!(alice.migrations_in_folder()?, &["init"]);
    };

    // Alice merges Bob's changes
    {
        alice.merge_from(&bob)?;

        assert_eq!(alice.migrations_in_folder()?, &["add_dogs", "init"]);
    };

    // Bob runs `prisma2 migrate up`.
    {
        assert!(alice.list_migrations().await?.last().is_none());
        assert!(bob.list_migrations().await?.last().is_none());

        bob.up().execute().await?;

        assert!(alice.list_migrations().await?.last().is_none());
        assert_eq!(bob.list_migrations().await?.last().unwrap().name, "add_dogs");
    }

    Ok(())
}

#[test_each_connector(log = "debug")]
async fn users_cannot_add_the_same_model_separately(api: &TestApi) -> TestResult {
    // Initial setup
    let master = {
        let initial_dm = r#"
            model Cat {
                id String @id
                name String
            }
        "#;

        let master = api.new_user("master", initial_dm).await?;

        master.save("01-init").execute().await?;
        master.up().execute().await?;

        master
    };

    let alice = api.user_cloned_from(&master, "alice").await?;
    let bob = api.user_cloned_from(&master, "bob").await?;

    // Alice creates the Dog model and merges to master
    {
        let dm2 = r#"
            model Cat {
                id String @id
                name String
            }

            model Dog {
                id String @id
                name String
            }
        "#;
        alice.write_schema(dm2)?;

        alice.save("02-create-dog").execute().await?;
        alice.up().execute().await?;

        master.merge_from(&alice)?;
        master.up().execute().await?;
    }

    // Bob creates his own Dog model and tries to merge into master
    {
        let dm2 = r#"
            model Cat {
                id String @id
                name String
            }

            model Dog {
                id String @id
                age Int
            }
        "#;
        bob.write_schema(dm2)?;

        bob.save("02-create-bobs-dog").execute().await?;
        bob.up().execute().await?;

        master.assert_cannot_merge(&bob)?;
    }

    assert_eq!(master.schema_string()?, alice.schema_string()?);
    assert_eq!(master.migrations_in_folder()?, &["01-init", "02-create-dog"]);

    master.save("03-uptodate-or-what").assert_is_up_to_date().await?;

    Ok(())
}

use crate::*;
use pretty_assertions::{assert_eq, assert_ne};

#[test_each_connector]
async fn mark_migration_applied_on_an_empty_database_works(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let persistence = api.imperative_migration_persistence();

    let dm = r#"
        model Test {
            id Int @id
        }
    "#;

    let output = api
        .create_migration("01init", dm, &migrations_directory)
        .send()
        .await?
        .into_output();

    let migration_name = output.generated_migration_name.unwrap();

    api.assert_schema().await?.assert_tables_count(0)?;

    assert!(
        persistence.list_migrations().await?.is_err(),
        "The migrations table should not be there yet."
    );

    api.mark_migration_applied(&migration_name, &migrations_directory)
        .expect_failed(false)
        .send()
        .await?;

    let applied_migrations = persistence.list_migrations().await?.unwrap();

    assert_eq!(applied_migrations.len(), 1);
    assert_eq!(&applied_migrations[0].migration_name, &migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_eq!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );

    api.assert_schema()
        .await?
        .assert_tables_count(1)?
        .assert_has_table("_prisma_migrations")?;

    Ok(())
}

#[test_each_connector]
async fn mark_migration_applied_on_an_empty_database_with_expect_failed_errors(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let persistence = api.imperative_migration_persistence();

    let dm = r#"
        model Test {
            id Int @id
        }
    "#;

    let output = api
        .create_migration("01init", dm, &migrations_directory)
        .send()
        .await?
        .into_output();

    let migration_name = output.generated_migration_name.unwrap();

    api.assert_schema().await?.assert_tables_count(0)?;

    assert!(
        persistence.list_migrations().await?.is_err(),
        "The migrations table should not be there yet."
    );

    let err = api
        .mark_migration_applied(&migration_name, &migrations_directory)
        .expect_failed(true)
        .send()
        .await
        .unwrap_err();

    assert_eq!(err.to_string(), "Generic error: Invariant violation: expect_failed was passed but no failed migration was found in the database.");

    assert!(persistence.list_migrations().await?.is_err());

    api.assert_schema().await?.assert_tables_count(0)?;

    Ok(())
}

#[test_each_connector]
async fn mark_migration_applied_on_a_non_empty_database_without_failed_works(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let persistence = api.imperative_migration_persistence();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = r#"
            model Test {
                id Int @id
            }
        "#;

        let output_initial_migration = api
            .create_migration("01init", dm1, &migrations_directory)
            .send()
            .await?
            .into_output();

        api.apply_migrations(&migrations_directory).send().await?;

        output_initial_migration.generated_migration_name.unwrap()
    };

    // Create a second migration
    let second_migration_name = {
        let dm2 = r#"
            model Test {
                id Int @id
            }

            model Cat {
                id Int @id
                name String
            }
        "#;

        let output_second_migration = api
            .create_migration("02migration", dm2, &migrations_directory)
            .send()
            .await?
            .into_output();

        output_second_migration.generated_migration_name.unwrap()
    };

    // Mark the second migration as applied

    api.mark_migration_applied(&second_migration_name, &migrations_directory)
        .expect_failed(false)
        .send()
        .await?;

    let applied_migrations = persistence.list_migrations().await?.unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_ne!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );
    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_some());
    assert_eq!(
        &applied_migrations[1].started_at,
        applied_migrations[1].finished_at.as_ref().unwrap()
    );

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("_prisma_migrations")?
        .assert_has_table("Test")?;

    Ok(())
}

#[test_each_connector]
async fn mark_migration_applied_on_a_non_empty_database_with_wrong_expect_failed(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let persistence = api.imperative_migration_persistence();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = r#"
            model Test {
                id Int @id
            }
        "#;

        let output_initial_migration = api
            .create_migration("01init", dm1, &migrations_directory)
            .send()
            .await?
            .into_output();

        api.apply_migrations(&migrations_directory).send().await?;

        output_initial_migration.generated_migration_name.unwrap()
    };

    // Create a second migration
    let second_migration_name = {
        let dm2 = r#"
            model Test {
                id Int @id
            }

            model Cat {
                id Int @id
                name String
            }
        "#;

        let output_second_migration = api
            .create_migration("02migration", dm2, &migrations_directory)
            .send()
            .await?
            .into_output();

        output_second_migration.generated_migration_name.unwrap()
    };

    // Mark the second migration as applied

    let error = api
        .mark_migration_applied(&second_migration_name, &migrations_directory)
        .expect_failed(true)
        .send()
        .await
        .unwrap_err();

    assert_eq!(error.to_string(), "Generic error: Invariant violation: expect_failed was passed but no failed migration was found in the database.");

    let applied_migrations = persistence.list_migrations().await?.unwrap();

    assert_eq!(applied_migrations.len(), 1);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_ne!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("_prisma_migrations")?
        .assert_has_table("Test")?;

    Ok(())
}

#[test_each_connector]
async fn mark_migration_applied_when_the_migration_is_already_applied_errors(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let persistence = api.imperative_migration_persistence();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = r#"
            model Test {
                id Int @id
            }
        "#;

        let output_initial_migration = api
            .create_migration("01init", dm1, &migrations_directory)
            .send()
            .await?
            .into_output();

        output_initial_migration.generated_migration_name.unwrap()
    };

    // Create a second migration
    let second_migration_name = {
        let dm2 = r#"
            model Test {
                id Int @id
            }

            model Cat {
                id Int @id
                name String
            }
        "#;

        let output_second_migration = api
            .create_migration("02migration", dm2, &migrations_directory)
            .send()
            .await?
            .into_output();

        output_second_migration.generated_migration_name.unwrap()
    };

    api.apply_migrations(&migrations_directory).send().await?;

    // Mark the second migration as applied again

    let err = api
        .mark_migration_applied(&second_migration_name, &migrations_directory)
        .expect_failed(false)
        .send()
        .await
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "The migration `{}` is already recorded as applied in the database.",
            second_migration_name
        )
    );

    let applied_migrations = persistence.list_migrations().await?.unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_ne!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );
    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_some());
    assert_ne!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );

    api.assert_schema()
        .await?
        .assert_tables_count(3)?
        .assert_has_table("_prisma_migrations")?
        .assert_has_table("Cat")?
        .assert_has_table("Test")?;

    Ok(())
}

#[test_each_connector]
async fn mark_migration_applied_when_the_migration_is_failed(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;
    let persistence = api.imperative_migration_persistence();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = r#"
            model Test {
                id Int @id
            }
        "#;

        let output_initial_migration = api
            .create_migration("01init", dm1, &migrations_directory)
            .send()
            .await?
            .into_output();

        output_initial_migration.generated_migration_name.unwrap()
    };

    // Create a second migration
    let second_migration_name = {
        let dm2 = r#"
            model Test {
                id Int @id
            }

            model Cat {
                id Int @id
                name String
            }
        "#;

        let output_second_migration = api
            .create_migration("02migration", dm2, &migrations_directory)
            .send()
            .await?
            .modify_migration(|migration| {
                migration.clear();
                migration.push_str("\nSELECT YOLO;");
            })?
            .into_output();

        output_second_migration.generated_migration_name.unwrap()
    };

    api.apply_migrations(&migrations_directory).send().await.ok();

    // Check that the second migration failed.
    {
        let applied_migrations = persistence.list_migrations().await?.unwrap();

        assert_eq!(applied_migrations.len(), 2);
        assert!(
            applied_migrations[1].finished_at.is_none(),
            "The second migration should fail."
        );
    }

    // Mark the second migration as applied again

    api.mark_migration_applied(&second_migration_name, &migrations_directory)
        .expect_failed(true)
        .send()
        .await?;

    let applied_migrations = persistence.list_migrations().await?.unwrap();

    assert_eq!(applied_migrations.len(), 3);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_ne!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );

    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_none());
    assert!(&applied_migrations[1].rolled_back_at.is_some());

    assert_eq!(&applied_migrations[2].migration_name, &second_migration_name);
    assert!(&applied_migrations[2].finished_at.is_some());
    assert!(&applied_migrations[2].rolled_back_at.is_none());

    api.assert_schema()
        .await?
        .assert_tables_count(2)?
        .assert_has_table("_prisma_migrations")?
        .assert_has_table("Test")?;

    Ok(())
}

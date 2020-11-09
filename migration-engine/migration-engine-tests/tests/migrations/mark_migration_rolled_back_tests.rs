use crate::*;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn mark_migration_rolled_back_on_an_empty_database_errors(api: &TestApi) -> TestResult {
    let err = api.mark_migration_rolled_back("anything").send().await.unwrap_err();

    assert_eq!(
        err.to_string(),
        "Invariant violation: called markMigrationRolledBack on a database without migrations table."
    );

    Ok(())
}

#[test_each_connector]
async fn mark_migration_rolled_back_on_a_database_with_migrations_table_errors(api: &TestApi) -> TestResult {
    api.imperative_migration_persistence().initialize().await?;

    let err = api.mark_migration_rolled_back("anything").send().await.unwrap_err();

    assert_eq!(
        err.to_string(),
        "Migration `anything` cannot be rolled back because it was never applied to the database."
    );

    Ok(())
}

#[test_each_connector]
async fn mark_migration_rolled_back_with_a_failed_migration_works(api: &TestApi) -> TestResult {
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
        assert!(
            applied_migrations[1].rolled_back_at.is_none(),
            "The second migration should fail."
        );
    }

    // Mark the second migration as rolled back.

    api.mark_migration_rolled_back(&second_migration_name).send().await?;

    let applied_migrations = persistence.list_migrations().await?.unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_ne!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );

    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_none());
    assert!(&applied_migrations[1].rolled_back_at.is_some());

    Ok(())
}

#[test_each_connector]
async fn mark_migration_rolled_back_with_a_successful_migration_errors(api: &TestApi) -> TestResult {
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

    api.apply_migrations(&migrations_directory).send().await.ok();

    // Check that the second migration failed.
    {
        let applied_migrations = persistence.list_migrations().await?.unwrap();

        assert_eq!(applied_migrations.len(), 2);
        assert!(applied_migrations[1].finished_at.is_some(),);
        assert!(applied_migrations[1].rolled_back_at.is_none(),);
    }

    // Mark the second migration as rolled back.

    let err = api
        .mark_migration_rolled_back(&second_migration_name)
        .send()
        .await
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        format!(
            "Migration `{}` cannot be rolled back because it is not in a failed state.",
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
    assert!(&applied_migrations[1].rolled_back_at.is_none());

    Ok(())
}

#[test_each_connector(log = "debug")]
async fn rolling_back_applying_again_then_rolling_back_again_should_error(api: &TestApi) -> TestResult {
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
    let (second_migration_name, second_migration_assertions) = {
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
            })?;

        (
            output_second_migration
                .output()
                .generated_migration_name
                .clone()
                .unwrap(),
            output_second_migration,
        )
    };

    api.apply_migrations(&migrations_directory).send().await.ok();

    // Check that the second migration failed.
    {
        let applied_migrations = persistence.list_migrations().await?.unwrap();

        assert_eq!(applied_migrations.len(), 2);
        assert!(applied_migrations[1].finished_at.is_none());
        assert!(applied_migrations[1].rolled_back_at.is_none());
    }

    // Mark the second migration as rolled back.
    api.mark_migration_rolled_back(&second_migration_name).send().await?;

    // Fix the migration
    second_migration_assertions.modify_migration(|migration| {
        migration.clear();
        migration.push_str("SELECT 'YOLO'");
    })?;

    // Reapply migration 2
    api.apply_migrations(&migrations_directory).send().await?;

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

    // Try to mark the second migration as rolled back again.
    api.mark_migration_rolled_back(&second_migration_name).send().await?;

    let final_migrations = persistence.list_migrations().await?.unwrap();

    // Assert that the last two migration records did not change, except for things like checksums.
    assert_eq!(&final_migrations[1].migration_name, &second_migration_name);
    assert_eq!(&final_migrations[1].finished_at, &applied_migrations[1].finished_at);
    assert_eq!(
        &final_migrations[1].rolled_back_at,
        &applied_migrations[1].rolled_back_at
    );

    assert_eq!(&final_migrations[2].migration_name, &second_migration_name);
    assert_eq!(&final_migrations[2].finished_at, &applied_migrations[2].finished_at);
    assert_eq!(
        &final_migrations[2].rolled_back_at,
        &applied_migrations[2].rolled_back_at
    );

    Ok(())
}

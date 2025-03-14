use pretty_assertions::assert_eq;
use sql_migration_tests::test_api::*;
use user_facing_errors::UserFacingError;

#[test_connector]
fn mark_migration_rolled_back_on_an_empty_database_errors(api: TestApi) {
    let err = api.mark_migration_rolled_back("anything").send_unwrap_err();

    assert!(err
        .to_string()
        .starts_with("Invariant violation: called markMigrationRolledBack on a database without migrations table.\n"));
}

#[test_connector]
fn mark_migration_rolled_back_on_a_database_with_migrations_table_errors(api: TestApi) {
    tok(api.migration_persistence().initialize(None)).unwrap();

    let err = api
        .mark_migration_rolled_back("anything")
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    assert_eq!(
        err.error_code,
        user_facing_errors::schema_engine::CannotRollBackUnappliedMigration::ERROR_CODE
    );

    assert_eq!(
        err.message,
        "Migration `anything` cannot be rolled back because it was never applied to the database. Hint: did you pass in the whole migration name? (example: \"20201207184859_initial_migration\")"
    );
}

#[test_connector]
fn mark_migration_rolled_back_with_a_failed_migration_works(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = api.datamodel_with_provider(
            r#"
            model Test {
                id Int @id
            }
        "#,
        );

        let output_initial_migration = api
            .create_migration("01init", &dm1, &migrations_directory)
            .send_sync()
            .into_output();

        output_initial_migration.generated_migration_name
    };

    // Create a second migration
    let second_migration_name = {
        let dm2 = api.datamodel_with_provider(
            r#"
            model Test {
                id Int @id
            }

            model Cat {
                id Int @id
                name String
            }
        "#,
        );

        let output_second_migration = api
            .create_migration("02migration", &dm2, &migrations_directory)
            .send_sync()
            .modify_migration(|migration| {
                migration.clear();
                migration.push_str("\nSELECT YOLO;");
            })
            .into_output();

        output_second_migration.generated_migration_name
    };

    api.apply_migrations(&migrations_directory).send_unwrap_err();

    // Check that the second migration failed.
    {
        let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

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

    api.mark_migration_rolled_back(&second_migration_name).send();

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());

    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_none());
    assert!(&applied_migrations[1].rolled_back_at.is_some());
}

#[test_connector]
fn mark_migration_rolled_back_with_a_successful_migration_errors(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = api.datamodel_with_provider(
            r#"
            model Test {
                id Int @id
            }
        "#,
        );

        let output_initial_migration = api
            .create_migration("01init", &dm1, &migrations_directory)
            .send_sync()
            .into_output();

        output_initial_migration.generated_migration_name
    };

    // Create a second migration
    let second_migration_name = {
        let dm2 = api.datamodel_with_provider(
            r#"
            model Test {
                id Int @id
            }

            model Cat {
                id Int @id
                name String
            }
        "#,
        );

        let output_second_migration = api
            .create_migration("02migration", &dm2, &migrations_directory)
            .send_sync()
            .into_output();

        output_second_migration.generated_migration_name
    };

    api.apply_migrations(&migrations_directory).send_sync();

    // Check that the second migration succeeded.
    {
        let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

        assert_eq!(applied_migrations.len(), 2);
        assert!(applied_migrations[1].finished_at.is_some(),);
        assert!(applied_migrations[1].rolled_back_at.is_none(),);
    }

    // Mark the second migration as rolled back.

    let err = api.mark_migration_rolled_back(&second_migration_name).send_unwrap_err();

    assert!(err.to_string().starts_with(&format!(
        "Migration `{second_migration_name}` cannot be rolled back because it is not in a failed state.\n"
    )));

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());

    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_some());
    assert!(&applied_migrations[1].rolled_back_at.is_none());
}

#[test_connector]
fn rolling_back_applying_again_then_rolling_back_again_should_error(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    // Create and apply a first migration
    let initial_migration_name = {
        let dm1 = api.datamodel_with_provider(
            r#"
             model Test {
                 id Int @id
             }
         "#,
        );

        let output_initial_migration = api
            .create_migration("01init", &dm1, &migrations_directory)
            .send_sync()
            .into_output();

        output_initial_migration.generated_migration_name
    };

    // Create a second migration
    let dm2 = api.datamodel_with_provider(
        r#"
             model Test {
                 id Int @id
             }

             model Cat {
                 id Int @id
                 name String
             }
         "#,
    );

    let (second_migration_name, second_migration_path) = {
        let output_second_migration = api
            .create_migration("02migration", &dm2, &migrations_directory)
            .send_sync()
            .modify_migration(|migration| {
                migration.clear();
                migration.push_str("\nSELECT YOLO;");
            });

        (
            output_second_migration.output().generated_migration_name.clone(),
            output_second_migration.migration_script_path(),
        )
    };

    api.apply_migrations(&migrations_directory).send_unwrap_err();

    // Check that the second migration failed.
    {
        let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

        assert_eq!(applied_migrations.len(), 2);
        assert!(applied_migrations[1].finished_at.is_none());
        assert!(applied_migrations[1].rolled_back_at.is_none());
    }

    // Mark the second migration as rolled back.
    api.mark_migration_rolled_back(&second_migration_name).send();

    // Fix the migration
    std::fs::write(second_migration_path, "SELECT 'YOLO'").unwrap();

    // Reapply migration 2
    api.apply_migrations(&migrations_directory).send_sync();

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 3);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());

    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_none());
    assert!(&applied_migrations[1].rolled_back_at.is_some());

    assert_eq!(&applied_migrations[2].migration_name, &second_migration_name);
    assert!(&applied_migrations[2].finished_at.is_some());
    assert!(&applied_migrations[2].rolled_back_at.is_none());

    // Try to mark the second migration as rolled back again.
    api.mark_migration_rolled_back(&second_migration_name).send();

    let final_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

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
}

use pretty_assertions::assert_eq;
use sql_migration_tests::test_api::*;
use user_facing_errors::{UserFacingError, schema_engine::MigrationToMarkAppliedNotFound};

const BASE_DM: &str = r#"
    model Test {
        id Int @id
    }
"#;

#[test_connector]
fn mark_migration_applied_on_an_empty_database_works(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let output = api
        .create_migration("01init", &api.datamodel_with_provider(BASE_DM), &migrations_directory)
        .send_sync()
        .into_output();

    let migration_name = output.generated_migration_name;

    api.assert_schema().assert_tables_count(0);

    assert!(
        tok(api.migration_persistence().list_migrations()).unwrap().is_err(),
        "The migrations table should not be there yet."
    );

    api.mark_migration_applied(&migration_name, &migrations_directory)
        .send();

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 1);
    assert_eq!(&applied_migrations[0].migration_name, &migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_eq!(
        &applied_migrations[0].started_at,
        applied_migrations[0].finished_at.as_ref().unwrap()
    );

    api.assert_schema()
        .assert_tables_count(1)
        .assert_has_table("_prisma_migrations");
}

#[test_connector]
fn mark_migration_applied_on_a_non_empty_database_works(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    // Create and apply a first migration
    let initial_migration_name = {
        let output_initial_migration = api
            .create_migration("01init", &api.datamodel_with_provider(BASE_DM), &migrations_directory)
            .send_sync()
            .into_output();

        api.apply_migrations(&migrations_directory).send_sync();

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

    // Mark the second migration as applied

    api.mark_migration_applied(&second_migration_name, &migrations_directory)
        .send();

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_some());
    assert_eq!(
        &applied_migrations[1].started_at,
        applied_migrations[1].finished_at.as_ref().unwrap()
    );

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("_prisma_migrations")
        .assert_has_table("Test");
}

#[test_connector]
fn mark_migration_applied_when_the_migration_is_already_applied_errors(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    // Create and apply a first migration
    let initial_migration_name = {
        let output_initial_migration = api
            .create_migration("01init", &api.datamodel_with_provider(BASE_DM), &migrations_directory)
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

    // Mark the second migration as applied again

    let err = api
        .mark_migration_applied(&second_migration_name, &migrations_directory)
        .send_unwrap_err();

    assert!(err.to_string().starts_with(&format!(
        "The migration `{second_migration_name}` is already recorded as applied in the database.\n"
    )));

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 2);
    assert_eq!(&applied_migrations[0].migration_name, &initial_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());
    assert_eq!(&applied_migrations[1].migration_name, &second_migration_name);
    assert!(&applied_migrations[1].finished_at.is_some());

    api.assert_schema()
        .assert_tables_count(3)
        .assert_has_table("_prisma_migrations")
        .assert_has_table("Cat")
        .assert_has_table("Test");
}

#[test_connector]
fn mark_migration_applied_when_the_migration_is_failed(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    // Create and apply a first migration
    let initial_migration_name = {
        let output_initial_migration = api
            .create_migration("01init", &api.datamodel_with_provider(BASE_DM), &migrations_directory)
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
    }

    // Mark the second migration as applied again

    api.mark_migration_applied(&second_migration_name, &migrations_directory)
        .send();

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

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("_prisma_migrations")
        .assert_has_table("Test");
}

#[test_connector]
fn baselining_should_work(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model test {
            id Int @id
        }
    "#,
    );

    api.schema_push(dm1.clone()).send();

    // Create a first local migration that matches the db contents
    let baseline_migration_name = {
        let output_baseline_migration = api
            .create_migration("01baseline", &dm1, &migrations_directory)
            .send_sync()
            .into_output();

        output_baseline_migration.generated_migration_name
    };

    // Mark the baseline migration as applied
    api.mark_migration_applied(&baseline_migration_name, &migrations_directory)
        .send();

    let applied_migrations = tok(api.migration_persistence().list_migrations()).unwrap().unwrap();

    assert_eq!(applied_migrations.len(), 1);
    assert_eq!(&applied_migrations[0].migration_name, &baseline_migration_name);
    assert!(&applied_migrations[0].finished_at.is_some());

    api.assert_schema()
        .assert_tables_count(2)
        .assert_has_table("_prisma_migrations")
        .assert_has_table("test");
}

#[test_connector]
fn must_return_helpful_error_on_migration_not_found(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let output = api
        .create_migration("01init", &api.datamodel_with_provider(BASE_DM), &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(1)
        .into_output();

    let migration_name = output.generated_migration_name;

    let err = api
        .mark_migration_applied("01init", &migrations_directory)
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    assert_eq!(err.error_code, MigrationToMarkAppliedNotFound::ERROR_CODE);

    api.mark_migration_applied(migration_name, &migrations_directory).send();
}

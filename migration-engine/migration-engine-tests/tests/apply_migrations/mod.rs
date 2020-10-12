use crate::*;
use pretty_assertions::assert_eq;
use user_facing_errors::UserFacingError;

#[test_each_connector]
async fn apply_migrations_with_an_empty_migrations_folder_works(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    Ok(())
}

#[test_each_connector]
async fn applying_a_single_migration_should_work(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id Int @id
            name String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("init", dm, &migrations_directory).send().await?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["init"])?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    Ok(())
}

#[test_each_connector]
async fn applying_two_migrations_works(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm1, &migrations_directory)
        .send()
        .await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &migrations_directory)
        .send()
        .await?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial", "second-migration"])?;

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    Ok(())
}

#[test_each_connector]
async fn migrations_should_fail_when_the_script_is_invalid(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    api.create_migration("initial", dm1, &migrations_directory)
        .send()
        .await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &migrations_directory)
        .send()
        .await?
        .modify_migration(|contents| contents.push_str("\nSELECT (^.^)_n;\n"))?;

    let result = api.apply_migrations(&migrations_directory).send().await;

    assert!(result.is_err());

    let mut migrations = api.imperative_migration_persistence().list_migrations().await?.unwrap();

    assert_eq!(migrations.len(), 2);

    let second = migrations.pop().unwrap();
    let first = migrations.pop().unwrap();

    let first = first.assert_migration_name("initial")?.assert_applied_steps_count(1)?;
    assert!(!first.is_failed());

    assert!(second.is_failed());
    second
        .assert_migration_name("second-migration")?
        .assert_applied_steps_count(0)?;

    Ok(())
}

#[test_each_connector]
async fn migrations_should_fail_to_apply_if_modified(api: &TestApi) -> TestResult {
    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let migrations_directory = api.create_migrations_directory()?;

    let assertions = api
        .create_migration("initial", dm1, &migrations_directory)
        .send()
        .await?;

    api.apply_migrations(&migrations_directory).send().await?;

    assertions.modify_migration(|script| script.push_str("/* this is just a harmless comment */"))?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &migrations_directory)
        .send()
        .await?;

    let err = api
        .apply_migrations(&migrations_directory)
        .send()
        .await
        .unwrap_err()
        .to_string();

    assert!(
        err.contains("The following migrations scripts are different from those that were applied to the database"),
        err
    );

    let mut migrations = api.imperative_migration_persistence().list_migrations().await?.unwrap();

    assert_eq!(migrations.len(), 1);
    let migration = migrations.pop().unwrap();

    migration.assert_migration_name("initial")?;

    Ok(())
}

#[test_each_connector]
async fn migrations_should_fail_on_an_uninitialized_nonempty_database(api: &TestApi) -> TestResult {
    let dm = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.schema_push(dm).send().await?.assert_green()?;

    let directory = api.create_migrations_directory()?;

    api.create_migration("01-init", dm, &directory)
        .send()
        .await?
        .assert_migration_directories_count(1)?;

    let known_error = api
        .apply_migrations(&directory)
        .send()
        .await
        .unwrap_err()
        .render_user_facing()
        .unwrap_known();

    assert_eq!(
        known_error.error_code,
        user_facing_errors::migration_engine::DatabaseSchemaNotEmpty::ERROR_CODE
    );

    Ok(())
}

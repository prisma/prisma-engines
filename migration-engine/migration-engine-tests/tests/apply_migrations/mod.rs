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

    first
        .assert_migration_name("initial")?
        .assert_applied_steps_count(1)?
        .assert_success()?;

    second
        .assert_migration_name("second-migration")?
        .assert_applied_steps_count(0)?
        .assert_failed()?;

    Ok(())
}

#[test_each_connector]
async fn migrations_should_not_reapply_modified_migrations(api: &TestApi) -> TestResult {
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

    api.apply_migrations(&migrations_directory)
        .send()
        .await?
        .assert_applied_migrations(&["second-migration"])?;

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

// Reference for the tables created by PostGIS: https://postgis.net/docs/manual-1.4/ch04.html#id418599
#[test_each_connector(tags("postgres"))]
async fn migrations_should_succeed_on_an_uninitialized_nonempty_database_with_postgis_tables(
    api: &TestApi,
) -> TestResult {
    let dm = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let create_spatial_ref_sys_table = "CREATE TABLE IF NOT EXISTS \"spatial_ref_sys\" ( id SERIAL PRIMARY KEY )";
    // The capitalized Geometry is intentional here, because we want the matching to be case-insensitive.
    let create_geometry_columns_table = "CREATE TABLE IF NOT EXiSTS \"Geometry_columns\" ( id SERIAL PRIMARY KEY )";

    api.database().raw_cmd(create_spatial_ref_sys_table).await?;
    api.database().raw_cmd(create_geometry_columns_table).await?;

    let directory = api.create_migrations_directory()?;

    api.create_migration("01-init", dm, &directory)
        .send()
        .await?
        .assert_migration_directories_count(1)?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["01-init"])?;

    Ok(())
}

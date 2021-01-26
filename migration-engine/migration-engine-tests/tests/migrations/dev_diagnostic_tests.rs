use crate::*;
use migration_core::{
    commands::{CreateMigrationOutput, DevDiagnosticInput, DevDiagnosticOutput},
    migration_api,
};
use pretty_assertions::assert_eq;
use user_facing_errors::{
    migration_engine::{MigrationDoesNotApplyCleanly, ShadowDbCreationError},
    UserFacingError,
};

#[test_each_connector]
async fn dev_diagnostic_on_an_empty_database_without_migration_returns_create_migration(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;
    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    assert!(action.is_create_migration());

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_after_two_migrations_happy_path(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send().await?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial", "second-migration"])?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    assert!(action.is_create_migration());

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_detects_drift(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial"])?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.schema_push(dm2).send().await?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    assert_eq!(
        action.as_reset(),
        Some("Drift detected: Your database schema is not in sync with your migration history.")
    );

    Ok(())
}

#[test_each_connector(ignore("postgres", "mssql_2017", "mssql_2019"))]
async fn dev_diagnostic_calculates_drift_in_presence_of_failed_migrations(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("01_initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id      Int @id
            name    String
        }

        model Dog {
            id Int @id
            fluffiness Float
        }
    "#;

    let migration_two = api
        .create_migration("02_add_dogs", dm2, &directory)
        .send()
        .await?
        .modify_migration(|migration| {
            migration.push_str("\nSELECT YOLO;");
        })?;

    let err = api.apply_migrations(&directory).send().await.unwrap_err().to_string();
    assert!(err.contains("yolo") || err.contains("YOLO"), err);

    let migration_two =
        migration_two.modify_migration(|migration| migration.truncate(migration.len() - "SELECT YOLO;".len()))?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    let migration_two_name = migration_two.into_output().generated_migration_name.unwrap();

    let expected_message = format!(
        "- The migration `{}` failed.\n- The migration `{}` was modified after it was applied.\n- Drift detected: Your database schema is not in sync with your migration history.\n",
        migration_two_name, migration_two_name,
    );

    assert_eq!(action.as_reset(), Some(expected_message.as_str()));

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_returns_create_migration_when_the_database_is_behind(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial"])?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send().await?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    assert!(action.is_create_migration());

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_can_detect_when_the_migrations_directory_is_behind(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    let name = api
        .create_migration("second-migration", dm2, &directory)
        .send()
        .await?
        .into_output()
        .generated_migration_name
        .unwrap();

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial", "second-migration"])?;

    let second_migration_folder_path = directory.path().join(&name);
    std::fs::remove_dir_all(&second_migration_folder_path)?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    assert_eq!(action.as_reset(), Some(format!("- Drift detected: Your database schema is not in sync with your migration history.\n- The following migration(s) are applied to the database but missing from the local migrations directory: {}\n", name)).as_deref());

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_can_detect_when_history_diverges(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let first_migration_name = api
        .create_migration("1-initial", dm1, &directory)
        .send()
        .await?
        .into_output()
        .generated_migration_name
        .unwrap();

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    let deleted_migration_name = api
        .create_migration("2-second-migration", dm2, &directory)
        .send()
        .await?
        .into_output()
        .generated_migration_name
        .unwrap();

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["1-initial", "2-second-migration"])?;

    let second_migration_folder_path = directory.path().join(&deleted_migration_name);
    std::fs::remove_dir_all(&second_migration_folder_path)?;

    let dm3 = r#"
        model Dog {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("3-create-dog", dm3, &directory)
        .draft(true)
        .send()
        .await?
        .assert_migration_directories_count(2)?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    let expected_message = format!(
        "- Drift detected: Your database schema is not in sync with your migration history.\n- The migrations recorded in the database diverge from the local migrations directory. Last common migration: `{}`. Migrations applied to the database but absent from the migrations directory are: {}\n",
        first_migration_name,
        deleted_migration_name,
    );

    assert_eq!(action.as_reset(), Some(expected_message.as_str()));

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_can_detect_edited_migrations(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let initial_assertions = api.create_migration("initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send().await?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial", "second-migration"])?;

    let initial_migration_name = initial_assertions
        .modify_migration(|script| {
            std::mem::swap(script, &mut format!("/* test */\n{}", script));
        })?
        .into_output()
        .generated_migration_name
        .unwrap();

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    let expected_message = format!(
        "The migration `{}` was modified after it was applied.",
        initial_migration_name
    );

    assert_eq!(action.as_reset(), Some(expected_message.as_str()));

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_reports_migrations_failing_to_apply_cleanly(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    let initial_assertions = api.create_migration("initial", dm1, &directory).send().await?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.create_migration("second-migration", dm2, &directory).send().await?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial", "second-migration"])?;

    let initial_migration_name = initial_assertions
        .modify_migration(|script| {
            script.push_str("SELECT YOLO;\n");
        })?
        .into_output()
        .generated_migration_name
        .unwrap();

    let err = api
        .dev_diagnostic(&directory)
        .send()
        .await
        .unwrap_err()
        .render_user_facing();

    let known_err = err.as_known().unwrap();

    assert_eq!(known_err.error_code, MigrationDoesNotApplyCleanly::ERROR_CODE);
    assert!(known_err.message.contains(initial_migration_name.as_str()));

    Ok(())
}

#[test_each_connector]
async fn dev_diagnostic_with_a_nonexistent_migrations_directory_works(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    std::fs::remove_dir(directory.path())?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();
    assert!(action.is_create_migration());

    Ok(())
}

#[test_each_connector]
async fn with_a_failed_migration(api: &TestApi) -> TestResult {
    let migrations_directory = api.create_migrations_directory()?;

    let dm = r#"
        model Test {
            id Int @id
        }
    "#;

    let CreateMigrationOutput {
        generated_migration_name,
    } = api
        .create_migration("01-init", dm, &migrations_directory)
        .send()
        .await?
        .assert_migration_directories_count(1)?
        .modify_migration(|migration| {
            migration.clear();
            migration.push_str("CREATE_BROKEN");
        })?
        .into_output();

    let err = api
        .apply_migrations(&migrations_directory)
        .send()
        .await
        .unwrap_err()
        .to_string();

    match api.sql_family() {
        SqlFamily::Mssql => assert!(err.contains("Could not find stored procedure"), err),
        _ => assert!(&err.contains("syntax"), err),
    }

    std::fs::remove_dir_all(
        migrations_directory
            .path()
            .join(generated_migration_name.as_ref().unwrap()),
    )?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&migrations_directory).send().await?.into_output();

    assert!(action.as_reset().unwrap().contains(&format!(
        "The migration `{}` failed.",
        generated_migration_name.unwrap()
    )));

    Ok(())
}

#[test_each_connector]
async fn with_an_invalid_unapplied_migration_should_report_it(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&["initial"])?;

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    let CreateMigrationOutput {
        generated_migration_name,
    } = api
        .create_migration("second-migration", dm2, &directory)
        .send()
        .await?
        .modify_migration(|script| {
            *script = "CREATE BROKEN".into();
        })?
        .into_output();

    let err = api
        .dev_diagnostic(&directory)
        .send()
        .await
        .unwrap_err()
        .render_user_facing()
        .unwrap_known();

    assert_eq!(err.error_code, MigrationDoesNotApplyCleanly::ERROR_CODE);
    assert!(err.message.starts_with(&format!(
        "Migration `{}` failed to apply cleanly to a temporary database. \nError:",
        generated_migration_name.unwrap()
    )));

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn drift_can_be_detected_without_migrations_table(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    api.apply_script("CREATE TABLE \"Cat\" (\nid SERIAL PRIMARY KEY\n);")
        .await?;

    let dm1 = r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    let DevDiagnosticOutput { action } = api.dev_diagnostic(&directory).send().await?.into_output();

    assert_eq!(
        action.as_reset(),
        Some("Drift detected: Your database schema is not in sync with your migration history.")
    );

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn dev_diagnostic_shadow_database_creation_error_is_special_cased_mysql(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    api.database()
        .raw_cmd(&format!(
            "
            DROP USER IF EXISTS 'prismashadowdbtestuser';
            CREATE USER 'prismashadowdbtestuser' IDENTIFIED by '1234batman';
            GRANT ALL PRIVILEGES ON {}.* TO 'prismashadowdbtestuser';
            ",
            api.connection_info().dbname().unwrap(),
        ))
        .await?;

    let (host, port) = db_host_and_port_mysql_8_0();

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "mysql"
            url = "mysql://prismashadowdbtestuser:1234batman@{dbhost}:{dbport}/{dbname}"
        }}
        "#,
        dbhost = host,
        dbname = api.connection_info().dbname().unwrap(),
        dbport = port,
    );

    let migration_api = migration_api(&datamodel).await?;

    let err = migration_api
        .dev_diagnostic(&DevDiagnosticInput {
            migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
        })
        .await
        .unwrap_err()
        .render_user_facing()
        .unwrap_known();

    assert_eq!(err.error_code, ShadowDbCreationError::ERROR_CODE);
    assert!(err.message.starts_with("Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases.  More info: https://pris.ly/d/migrate-shadow."));

    Ok(())
}

#[test_each_connector(tags("postgres_12"), log = "debug")]
async fn dev_diagnostic_shadow_database_creation_error_is_special_cased_postgres(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    api.database()
        .raw_cmd(
            "
            DROP USER IF EXISTS prismashadowdbtestuser;
            CREATE USER prismashadowdbtestuser PASSWORD '1234batman' LOGIN;
            ",
        )
        .await?;

    let (host, port) = db_host_and_port_postgres_12();

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "postgresql"
            url = "postgresql://prismashadowdbtestuser:1234batman@{dbhost}:{dbport}/{dbname}"
        }}
        "#,
        dbhost = host,
        dbname = api.connection_info().dbname().unwrap(),
        dbport = port,
    );

    let migration_api = migration_api(&datamodel).await?;

    let err = migration_api
        .dev_diagnostic(&DevDiagnosticInput {
            migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
        })
        .await
        .unwrap_err()
        .render_user_facing()
        .unwrap_known();

    assert_eq!(err.error_code, ShadowDbCreationError::ERROR_CODE);
    assert!(err.message.starts_with("Prisma Migrate could not create the shadow database. Please make sure the database user has permission to create databases.  More info: https://pris.ly/d/migrate-shadow."));

    Ok(())
}

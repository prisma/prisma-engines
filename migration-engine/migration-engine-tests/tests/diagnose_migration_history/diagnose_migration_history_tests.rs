use crate::*;
use enumflags2::BitFlags;
use migration_core::{
    commands::{
        CreateMigrationOutput, DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic,
        HistoryDiagnostic,
    },
    migration_api,
};
use pretty_assertions::assert_eq;
use user_facing_errors::{migration_engine::ShadowDbCreationError, UserFacingError};

#[test_each_connector]
async fn diagnose_migrations_history_on_an_empty_database_without_migration_returns_nothing(
    api: &TestApi,
) -> TestResult {
    let directory = api.create_migrations_directory()?;
    let output = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(output.is_empty());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_after_two_migrations_happy_path(api: &TestApi) -> TestResult {
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

    let output = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(output.is_empty());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migration_history_with_opt_in_to_shadow_database_calculates_drift(api: &TestApi) -> TestResult {
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

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    let expected_rollback_warnings =
         "/*\n  Warnings:\n\n  - You are about to drop the column `fluffiness` on the `Cat` table. All the data in the column will be lost.\n\n*/";

    let rollback = drift.unwrap().unwrap_drift_detected();

    assert!(rollback.starts_with(expected_rollback_warnings), rollback);

    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migration_history_without_opt_in_to_shadow_database_does_not_calculate_drift(
    api: &TestApi,
) -> TestResult {
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

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector(ignore("postgres", "mssql_2017", "mssql_2019"))]
async fn diagnose_migration_history_calculates_drift_in_presence_of_failed_migrations(api: &TestApi) -> TestResult {
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

    migration_two.modify_migration(|migration| migration.truncate(migration.len() - "SELECT YOLO;".len()))?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    let rollback = drift.unwrap().unwrap_drift_detected();

    let expected_rollback_warnings = indoc::indoc!(
        "
        /*
          Warnings:

          - You are about to drop the `Dog` table. If the table is not empty, all the data it contains will be lost.

        */
        "
    );

    assert!(rollback.starts_with(expected_rollback_warnings), rollback);

    assert!(history.is_none());
    assert_eq!(failed_migration_names.len(), 1);
    assert_eq!(edited_migration_names.len(), 1);
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_can_detect_when_the_database_is_behind(api: &TestApi) -> TestResult {
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

    let name = api
        .create_migration("second-migration", dm2, &directory)
        .send()
        .await?
        .into_output()
        .generated_migration_name
        .unwrap();

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert_eq!(
        history,
        Some(HistoryDiagnostic::DatabaseIsBehind {
            unapplied_migration_names: vec![name],
        })
    );
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_can_detect_when_the_folder_is_behind(api: &TestApi) -> TestResult {
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

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(matches!(drift, Some(DriftDiagnostic::DriftDetected { rollback: _ })));
    assert_eq!(
        history,
        Some(HistoryDiagnostic::MigrationsDirectoryIsBehind {
            unpersisted_migration_names: vec![name],
        })
    );
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_can_detect_when_history_diverges(api: &TestApi) -> TestResult {
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

    let unapplied_migration_name = api
        .create_migration("3-create-dog", dm3, &directory)
        .draft(true)
        .send()
        .await?
        .assert_migration_directories_count(2)?
        .into_output()
        .generated_migration_name
        .unwrap();

    let DiagnoseMigrationHistoryOutput {
        history,
        drift,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(matches!(drift, Some(DriftDiagnostic::DriftDetected { rollback: _ })));
    assert_eq!(
        history,
        Some(HistoryDiagnostic::HistoriesDiverge {
            unapplied_migration_names: vec![unapplied_migration_name],
            unpersisted_migration_names: vec![deleted_migration_name],
            last_common_migration_name: Some(first_migration_name),
        })
    );
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_can_detect_edited_migrations(api: &TestApi) -> TestResult {
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

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        edited_migration_names,
        failed_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert_eq!(edited_migration_names, &[initial_migration_name]);
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_reports_migrations_failing_to_apply_cleanly(api: &TestApi) -> TestResult {
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

    let DiagnoseMigrationHistoryOutput {
        failed_migration_names,
        edited_migration_names,
        history,
        drift,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    assert!(has_migrations_table);
    assert_eq!(edited_migration_names, &[initial_migration_name.as_str()]);
    assert!(failed_migration_names.is_empty());
    assert_eq!(history, None);
    assert!(error_in_unapplied_migration.is_none());

    match drift {
        Some(DriftDiagnostic::MigrationFailedToApply { error }) => {
            let known_error = error.unwrap_known();
            assert_eq!(
                known_error.error_code,
                user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly::ERROR_CODE
            );
            assert_eq!(known_error.meta["migration_name"], initial_migration_name.as_str());
            assert!(known_error.message.contains("yolo") || known_error.message.contains("YOLO"));
        }
        _ => panic!("assertion failed"),
    }

    Ok(())
}

#[test_each_connector]
async fn diagnose_migrations_history_with_a_nonexistent_migrations_directory_works(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    std::fs::remove_dir(directory.path())?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        edited_migration_names,
        failed_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(!has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

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

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&migrations_directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert_eq!(failed_migration_names, &[generated_migration_name.unwrap()]);

    let error_in_unapplied_migration =
        error_in_unapplied_migration.expect("No error in unapplied migrations, but we expected one.");

    let message = error_in_unapplied_migration.message().to_owned();

    assert!(
        message.contains("01-init` failed to apply cleanly to a temporary database."),
        message,
    );
    assert_eq!(
        error_in_unapplied_migration.unwrap_known().error_code,
        user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly::ERROR_CODE,
    );

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

    let DiagnoseMigrationHistoryOutput {
        failed_migration_names,
        edited_migration_names,
        history,
        drift,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    assert!(has_migrations_table);
    assert!(edited_migration_names.is_empty());
    assert!(failed_migration_names.is_empty());
    assert!(
        matches!(history, Some(HistoryDiagnostic::DatabaseIsBehind { unapplied_migration_names: names }) if names == &[generated_migration_name.unwrap()])
    );
    assert!(drift.is_none());

    let error_in_unapplied_migration =
        error_in_unapplied_migration.expect("No error in unapplied migrations, but we expected one.");

    let message = error_in_unapplied_migration.message().to_owned();

    assert!(
        message.contains("_second-migration` failed to apply cleanly to a temporary database."),
        message,
    );
    assert_eq!(
        error_in_unapplied_migration.unwrap_known().error_code,
        user_facing_errors::migration_engine::MigrationDoesNotApplyCleanly::ERROR_CODE,
    );

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

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        edited_migration_names,
        failed_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send()
        .await?
        .into_output();

    assert!(matches!(drift, Some(DriftDiagnostic::DriftDetected { rollback: _ })));
    assert!(
        matches!(history, Some(HistoryDiagnostic::DatabaseIsBehind { unapplied_migration_names: migs }) if migs.len() == 1)
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(!has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn shadow_database_creation_error_is_special_cased_mysql(api: &TestApi) -> TestResult {
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

    let migration_api = migration_api(&datamodel, BitFlags::empty()).await?;

    let output = migration_api
        .diagnose_migration_history(&DiagnoseMigrationHistoryInput {
            migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
            opt_in_to_shadow_database: true,
        })
        .await?;

    assert!(
        matches!(output.drift, Some(DriftDiagnostic::MigrationFailedToApply { error }) if error.as_known().unwrap().error_code == ShadowDbCreationError::ERROR_CODE)
    );

    Ok(())
}

#[test_each_connector(tags("postgres_12"), log = "debug")]
async fn shadow_database_creation_error_is_special_cased_postgres(api: &TestApi) -> TestResult {
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

    let migration_api = migration_api(&datamodel, BitFlags::empty()).await?;

    let output = migration_api
        .diagnose_migration_history(&DiagnoseMigrationHistoryInput {
            migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
            opt_in_to_shadow_database: true,
        })
        .await?;

    assert!(
        matches!(output.drift, Some(DriftDiagnostic::MigrationFailedToApply { error }) if error.as_known().unwrap().error_code == ShadowDbCreationError::ERROR_CODE)
    );

    Ok(())
}

#[test_each_connector(tags("mssql_2019"))]
async fn shadow_database_creation_error_is_special_cased_mssql(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    let dm1 = r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#;

    api.create_migration("initial", dm1, &directory).send().await?;

    api.database().raw_cmd("DROP LOGIN prismashadowdbtestuser;").await.ok();

    api.database()
        .raw_cmd(
            "
            DROP USER IF EXISTS prismashadowdbtestuser;

            CREATE LOGIN prismashadowdbtestuser
                WITH PASSWORD = '1234batmanZ';

            CREATE USER prismashadowdbtestuser FOR LOGIN prismashadowdbtestuser;
            ",
        )
        .await?;

    let (host, port) = db_host_and_port_mssql_2019();

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "sqlserver"
            url = "sqlserver://{dbhost}:{dbport};database={dbname};user=prismashadowdbtestuser;password=1234batmanZ;trustservercertificate=true"
        }}
        "#,
        dbhost = host,
        dbname = api.connection_info().dbname().unwrap(),
        dbport = port,
    );

    let migration_api = migration_api(&datamodel, BitFlags::empty()).await?;

    let output = migration_api
        .diagnose_migration_history(&DiagnoseMigrationHistoryInput {
            migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
            opt_in_to_shadow_database: true,
        })
        .await?;

    assert!(
        matches!(output.drift, Some(DriftDiagnostic::MigrationFailedToApply { error }) if error.as_known().unwrap().error_code == ShadowDbCreationError::ERROR_CODE)
    );

    Ok(())
}

use pretty_assertions::assert_eq;
use schema_core::{
    commands::{DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic, HistoryDiagnostic},
    json_rpc::types::CreateMigrationOutput,
    schema_api,
};
use sql_migration_tests::test_api::*;
use std::io::Write;
use user_facing_errors::{schema_engine::ShadowDbCreationError, UserFacingError};

#[test_connector]
fn diagnose_migrations_history_on_an_empty_database_without_migration_returns_nothing(api: TestApi) {
    let directory = api.create_migrations_directory();
    let output = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(output.is_empty());
}

#[test_connector]
fn diagnose_migrations_history_after_two_migrations_happy_path(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let output = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(output.is_empty());
}

#[test_connector(tags(Postgres))]
fn diagnose_migration_history_with_opt_in_to_shadow_database_calculates_drift(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#;

    api.create_migration("initial", dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.schema_push_w_datasource(dm2).send();

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
        .send_sync()
        .into_output();

    let rollback = drift.unwrap().unwrap_drift_detected();

    let snapshot = expect_test::expect![[r#"

        [*] Changed the `Cat` table
          [+] Added column `fluffiness`
    "#]];

    snapshot.assert_eq(&rollback);

    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector]
fn diagnose_migration_history_without_opt_in_to_shadow_database_does_not_calculate_drift(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#;

    api.schema_push_w_datasource(dm2).send();

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector(exclude(Postgres, Mssql))]
fn diagnose_migration_history_calculates_drift_in_presence_of_failed_migrations(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("01_initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }

        model Dog {
            id Int @id
            fluffiness Float
        }
    "#,
    );

    let mut original_migration = String::new();
    let migration_two = api
        .create_migration("02_add_dogs", &dm2, &directory)
        .send_sync()
        .modify_migration(|migration| {
            original_migration.push_str(migration);
            migration.push_str("\nSELECT YOLO;");
        })
        .migration_script_path();

    let err = api.apply_migrations(&directory).send_unwrap_err().to_string();
    assert!(err.contains("yolo") || err.contains("YOLO"), "{}", err);

    std::fs::write(migration_two, original_migration.as_bytes()).unwrap();

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
        .send_sync()
        .into_output();

    let summary = drift.unwrap().unwrap_drift_detected();

    assert!(summary.starts_with("\n[+] Added tables"), "{}", summary);

    assert!(history.is_none());
    assert_eq!(failed_migration_names.len(), 1);
    assert_eq!(edited_migration_names.len(), 1);
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector]
fn diagnose_migrations_history_can_detect_when_the_database_is_behind(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let name = api
        .create_migration("second-migration", &dm2, &directory)
        .send_sync()
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
    } = api.diagnose_migration_history(&directory).send_sync().into_output();

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
}

#[test_connector]
fn diagnose_migrations_history_can_detect_when_the_folder_is_behind(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let name = api
        .create_migration("second-migration", &dm2, &directory)
        .send_sync()
        .into_output()
        .generated_migration_name
        .unwrap();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let second_migration_folder_path = directory.path().join(&name);
    std::fs::remove_dir_all(second_migration_folder_path).unwrap();

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
        .send_sync()
        .into_output();

    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(matches!(drift, Some(DriftDiagnostic::DriftDetected { summary: _ })));
    assert_eq!(
        history,
        Some(HistoryDiagnostic::MigrationsDirectoryIsBehind {
            unpersisted_migration_names: vec![name],
        })
    );
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector]
fn diagnose_migrations_history_can_detect_when_history_diverges(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let first_migration_name = api
        .create_migration("1-initial", &dm1, &directory)
        .send_sync()
        .into_output()
        .generated_migration_name
        .unwrap();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let deleted_migration_name = api
        .create_migration("2-second-migration", &dm2, &directory)
        .send_sync()
        .into_output()
        .generated_migration_name
        .unwrap();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["1-initial", "2-second-migration"]);

    let second_migration_folder_path = directory.path().join(&deleted_migration_name);
    std::fs::remove_dir_all(second_migration_folder_path).unwrap();

    let dm3 = api.datamodel_with_provider(
        r#"
        model Dog {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let unapplied_migration_name = api
        .create_migration("3-create-dog", &dm3, &directory)
        .draft(true)
        .send_sync()
        .assert_migration_directories_count(2)
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
        .send_sync()
        .into_output();

    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(matches!(drift, Some(DriftDiagnostic::DriftDetected { summary: _ })));
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
}

#[test_connector]
fn diagnose_migrations_history_can_detect_edited_migrations(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let (initial_migration_output, initial_migration_path) = {
        let initial_assertions = api.create_migration("initial", &dm1, &directory).send_sync();
        let path = initial_assertions.migration_script_path();
        (initial_assertions.into_output(), path)
    };
    let initial_migration_name = initial_migration_output.generated_migration_name.unwrap();

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(initial_migration_path)
        .unwrap();
    file.write_all(b"/* test */").unwrap();

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        edited_migration_names,
        failed_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert_eq!(edited_migration_names, &[initial_migration_name]);
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector]
fn diagnose_migrations_history_reports_migrations_failing_to_apply_cleanly(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    let (initial_migration_name, initial_path) = {
        let out = api.create_migration("initial", &dm1, &directory).send_sync();
        let path = out.migration_script_path();
        (out.into_output().generated_migration_name.unwrap(), path)
    };

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    api.create_migration("second-migration", &dm2, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial", "second-migration"]);

    let mut file = std::fs::OpenOptions::new().append(true).open(initial_path).unwrap();
    file.write_all(b"SELECT YOLO;\n").unwrap();

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
        .send_sync()
        .into_output();

    assert!(has_migrations_table);
    assert_eq!(edited_migration_names, &[initial_migration_name.as_str()]);
    assert!(failed_migration_names.is_empty());
    assert_eq!(history, None);
    assert!(error_in_unapplied_migration.is_none());

    match drift {
        Some(DriftDiagnostic::MigrationFailedToApply { error }) => {
            let known_error = error.to_user_facing().unwrap_known();
            assert_eq!(
                known_error.error_code,
                user_facing_errors::schema_engine::MigrationDoesNotApplyCleanly::ERROR_CODE
            );
            assert_eq!(known_error.meta["migration_name"], initial_migration_name.as_str());
            assert!(
                known_error.message.contains("yolo")
                    || known_error.message.contains("YOLO")
                    || known_error.message.contains("(not available)")
            );
        }
        _ => panic!("assertion failed"),
    }
}

#[test_connector]
fn diagnose_migrations_history_with_a_nonexistent_migrations_directory_works(api: TestApi) {
    let directory = api.create_migrations_directory();

    std::fs::remove_dir(directory.path()).unwrap();

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        edited_migration_names,
        failed_migration_names,
        has_migrations_table,
        error_in_unapplied_migration,
    } = api.diagnose_migration_history(&directory).send_sync().into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(!has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector]
fn dmh_with_a_failed_migration(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model catsu {
            id Int @id
        }
    "#,
    );

    let CreateMigrationOutput {
        generated_migration_name,
    } = api
        .create_migration("01-init", &dm, &migrations_directory)
        .send_sync()
        .assert_migration_directories_count(1)
        .modify_migration(|migration| {
            migration.clear();
            migration.push_str("CREATE_BROKEN");
        })
        .into_output();

    let err = api
        .apply_migrations(&migrations_directory)
        .send_unwrap_err()
        .to_string();

    if api.is_mssql() {
        assert!(err.contains("Could not find stored procedure"), "{}", err)
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
        .send_sync()
        .into_output();

    assert!(drift.is_none());
    assert!(history.is_none());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert_eq!(failed_migration_names, &[generated_migration_name.unwrap()]);

    let error_in_unapplied_migration = error_in_unapplied_migration
        .expect("No error in unapplied migrations, but we expected one.")
        .to_user_facing();

    let message = error_in_unapplied_migration.message().to_owned();

    assert!(
        message.contains("01-init` failed to apply cleanly to the shadow database."),
        "{}",
        message,
    );
    assert_eq!(
        error_in_unapplied_migration.unwrap_known().error_code,
        user_facing_errors::schema_engine::MigrationDoesNotApplyCleanly::ERROR_CODE,
    );
}

#[test_connector]
fn dmh_with_an_invalid_unapplied_migration_should_report_it(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            name    String
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let dm2 = api.datamodel_with_provider(
        r#"
        model Cat {
            id          Int @id
            name        String
            fluffiness  Float
        }
    "#,
    );

    let CreateMigrationOutput {
        generated_migration_name,
    } = api
        .create_migration("second-migration", &dm2, &directory)
        .send_sync()
        .modify_migration(|script| {
            *script = "CREATE BROKEN".into();
        })
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
        .send_sync()
        .into_output();

    assert!(has_migrations_table);
    assert!(edited_migration_names.is_empty());
    assert!(failed_migration_names.is_empty());
    assert!(
        matches!(history, Some(HistoryDiagnostic::DatabaseIsBehind { unapplied_migration_names: names }) if names == [generated_migration_name.unwrap()])
    );
    assert!(drift.is_none());

    let error_in_unapplied_migration = error_in_unapplied_migration
        .expect("No error in unapplied migrations, but we expected one.")
        .to_user_facing();

    let message = error_in_unapplied_migration.message().to_owned();

    assert!(
        message.contains("_second-migration` failed to apply cleanly to the shadow database."),
        "{}",
        message,
    );
    assert_eq!(
        error_in_unapplied_migration.unwrap_known().error_code,
        user_facing_errors::schema_engine::MigrationDoesNotApplyCleanly::ERROR_CODE,
    );
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
fn drift_can_be_detected_without_migrations_table(api: TestApi) {
    let directory = api.create_migrations_directory();

    api.raw_cmd("CREATE TABLE \"Cat\" (\nid SERIAL PRIMARY KEY\n);");

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

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
        .send_sync()
        .into_output();

    assert!(matches!(drift, Some(DriftDiagnostic::DriftDetected { summary: _ })));
    assert!(
        matches!(history, Some(HistoryDiagnostic::DatabaseIsBehind { unapplied_migration_names: migs }) if migs.len() == 1)
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(!has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector(tags(Mysql8), exclude(Vitess))]
fn shadow_database_creation_error_is_special_cased_mysql(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.raw_cmd(&format!(
        "
            DROP USER IF EXISTS 'prismashadowdbtestuser2';
            CREATE USER 'prismashadowdbtestuser2' IDENTIFIED by '1234batman';
            GRANT ALL PRIVILEGES ON {}.* TO 'prismashadowdbtestuser2';
            ",
        api.connection_info().dbname().unwrap(),
    ));

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "mysql"
            url = "mysql://prismashadowdbtestuser2:1234batman@{dbhost}:{dbport}/{dbname}"
        }}
        "#,
        dbhost = api.connection_info().host(),
        dbname = api.connection_info().dbname().unwrap(),
        dbport = api.connection_info().port().unwrap_or(3306),
    );

    let migration_api = schema_api(Some(datamodel), None).unwrap();

    let output = tok(migration_api.diagnose_migration_history(DiagnoseMigrationHistoryInput {
        migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
        opt_in_to_shadow_database: true,
    }))
    .unwrap();

    assert!(
        matches!(output.drift, Some(DriftDiagnostic::MigrationFailedToApply { error }) if error.to_user_facing().as_known().unwrap().error_code == ShadowDbCreationError::ERROR_CODE)
    );
}

#[test_connector(tags(Postgres12))]
fn shadow_database_creation_error_is_special_cased_postgres(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.raw_cmd(
        "
            DROP USER IF EXISTS prismashadowdbtestuser2;
            CREATE USER prismashadowdbtestuser2 PASSWORD '1234batman' LOGIN;
            ",
    );

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "postgresql"
            url = "postgresql://prismashadowdbtestuser2:1234batman@{dbhost}:{dbport}/{dbname}"
        }}
        "#,
        dbhost = api.connection_info().host(),
        dbname = api.connection_info().dbname().unwrap(),
        dbport = api.connection_info().port().unwrap_or(5432),
    );

    let output = tok(async {
        schema_api(Some(datamodel.clone()), None)
            .unwrap()
            .diagnose_migration_history(DiagnoseMigrationHistoryInput {
                migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
                opt_in_to_shadow_database: true,
            })
            .await
    })
    .unwrap();

    assert!(
        matches!(output.drift, Some(DriftDiagnostic::MigrationFailedToApply { error }) if error.to_user_facing().as_known().unwrap().error_code == ShadowDbCreationError::ERROR_CODE)
    );
}

#[test_connector(tags(Mssql2019))]
fn shadow_database_creation_error_is_special_cased_mssql(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm1 = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id @default(autoincrement())
        }
    "#,
    );

    api.create_migration("initial", &dm1, &directory).send_sync();

    api.raw_cmd(
        "
            BEGIN TRY
                CREATE LOGIN prismashadowdbtestuser WITH PASSWORD = '1234batmanZ';
                GRANT SELECT TO prismashadowdbuser;
            END TRY
            BEGIN CATCH
            END CATCH;
            ",
    );

    let datamodel = format!(
        r#"
        datasource db {{
            provider = "sqlserver"
            url = "sqlserver://{dbhost}:{dbport};user=prismashadowdbtestuser;password=1234batmanZ;trustservercertificate=true"
        }}
        "#,
        dbhost = api.connection_info().host(),
        dbport = api.connection_info().port().unwrap(),
    );

    let mut tries = 0;

    let migration_api = loop {
        if tries > 5 {
            panic!("Failed to connect to mssql more than five times.");
        }

        let result = schema_api(Some(datamodel.clone()), None);

        match result {
            Ok(api) => break api,
            Err(err) => {
                eprintln!("got err, sleeping\nerr:{err:?}");
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
    };

    let output = tok(migration_api.diagnose_migration_history(DiagnoseMigrationHistoryInput {
        migrations_directory_path: directory.path().as_os_str().to_string_lossy().into_owned(),
        opt_in_to_shadow_database: true,
    }))
    .unwrap();

    assert!(
        matches!(output.drift, Some(DriftDiagnostic::MigrationFailedToApply { error }) if error.to_user_facing().as_known().unwrap().error_code == ShadowDbCreationError::ERROR_CODE)
    );
}

#[test_connector(tags(Sqlite))]
fn empty_migration_directories_should_cause_known_errors(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model Cat {
            id      Int @id
            hasBox  Boolean
        }
    "#,
    );

    let output = api
        .create_migration("01init", &dm, &migrations_directory)
        .send_sync()
        .into_output();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01init"]);

    let dirname = output.generated_migration_name.unwrap();
    let dirpath = migrations_directory.path().join(dirname);

    assert!(dirpath.exists());

    let filepath = dirpath.join("migration.sql");

    assert!(filepath.exists());

    std::fs::remove_file(&filepath).unwrap();

    let err = api
        .diagnose_migration_history(&migrations_directory)
        .send_unwrap_err()
        .to_user_facing()
        .unwrap_known();

    assert_eq!(
        err.error_code,
        user_facing_errors::schema_engine::MigrationFileNotFound::ERROR_CODE
    );

    assert_eq!(
        err.meta,
        serde_json::json!({ "migration_file_path": filepath.to_string_lossy(), })
    );
}

#[test_connector]
fn indexes_on_same_columns_with_different_names_should_work(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model a {
            users_id Int
            roles_id Int

            @@id([users_id, roles_id])
            @@unique([users_id, roles_id], name: "unique_constraint")
            @@index([users_id, roles_id], name: "users_has_roles.users_id_roles_id_index")
            @@index([users_id, roles_id], name: "users_id_with_roles_id_index")
        }
    "#,
    );

    api.create_migration("initial", &dm, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let output = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();

    assert!(output.drift.is_none());
}

#[test_connector(exclude(Sqlite, Mssql))]
fn foreign_keys_on_same_columns_should_work(api: TestApi) {
    let directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model prisma_bug_1 {
            id1            BigInt
            id2            BigInt
            prisma_bug_2_a prisma_bug_2[] @relation("a")
            prisma_bug_2_b prisma_bug_2[] @relation("b")

            @@id([id1, id2])
          }

          model prisma_bug_2 {
            id BigInt @id

            prisma_bug_1_id1 BigInt
            prisma_bug_1_id2 BigInt

            prisma_bug_1_a prisma_bug_1  @relation("a", fields: [prisma_bug_1_id1, prisma_bug_1_id2], references: [id1, id2], map: "prisma_bug_1_a_fk")
            prisma_bug_1_b prisma_bug_1? @relation("b", fields: [prisma_bug_1_id1, prisma_bug_1_id2], references: [id1, id2], map: "prisma_bug_1_b_fk")
          }
    "#,
    );

    api.create_migration("initial", &dm, &directory).send_sync();

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&["initial"]);

    let output = api
        .diagnose_migration_history(&directory)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();

    assert!(output.drift.is_none());
    assert!(output.is_empty());
}

#[test_connector(tags(Postgres))]
fn default_dbgenerated_should_not_cause_drift(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model A {
            id String @id @default(dbgenerated("(now())::TEXT"))
        }
    "#,
    );

    api.create_migration("01init", &dm, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01init"]);

    let output = api
        .diagnose_migration_history(&migrations_directory)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();

    assert!(output.drift.is_none());
}

#[test_connector(tags(Postgres))]
fn default_uuid_should_not_cause_drift(api: TestApi) {
    let migrations_directory = api.create_migrations_directory();

    let dm = api.datamodel_with_provider(
        r#"
        model A {
            id   String @id @db.Uuid
            uuid String @db.Uuid @default("00000000-0000-0000-0016-000000000004")
        }
    "#,
    );

    api.create_migration("01init", &dm, &migrations_directory).send_sync();

    api.apply_migrations(&migrations_directory)
        .send_sync()
        .assert_applied_migrations(&["01init"]);

    let output = api
        .diagnose_migration_history(&migrations_directory)
        .opt_in_to_shadow_database(true)
        .send_sync()
        .into_output();

    assert!(output.drift.is_none());
}

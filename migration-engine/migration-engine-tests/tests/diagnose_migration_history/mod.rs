use crate::*;
use migration_core::commands::HistoryDiagnostic;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn diagnose_migrations_history_on_an_empty_database_without_migration_returns_nothing(
    api: &TestApi,
) -> TestResult {
    let directory = api.create_migrations_directory()?;
    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(result.history_problems, &[]);

    Ok(())
}

#[test_each_connector(log = "debug, sql_schema_describer=info")]
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(result.history_problems, &[]);

    Ok(())
}

#[test_each_connector]
async fn diagnose_migration_history_detects_drift(api: &TestApi) -> TestResult {
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();
    assert_eq!(result.history_problems, &[HistoryDiagnostic::DriftDetected]);

    Ok(())
}

#[test_each_connector(log = "debug, sql_schema_describer=info")]
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(
        result.history_problems,
        &[HistoryDiagnostic::DatabaseIsBehind {
            unapplied_migration_names: vec![name],
        }]
    );

    Ok(())
}

#[test_each_connector(log = "debug, sql_schema_describer=info")]
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(
        result.history_problems,
        &[
            HistoryDiagnostic::MigrationsDirectoryIsBehind {
                unpersisted_migration_names: vec![name],
            },
            HistoryDiagnostic::DriftDetected
        ]
    );

    Ok(())
}

#[test_each_connector(log = "debug, sql_schema_describer=info")]
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(
        result.history_problems,
        &[
            HistoryDiagnostic::HistoriesDiverge {
                unapplied_migration_names: vec![unapplied_migration_name],
                unpersisted_migration_names: vec![deleted_migration_name],
                last_common_migration_name: Some(first_migration_name),
            },
            HistoryDiagnostic::DriftDetected
        ]
    );

    Ok(())
}

#[test_each_connector(log = "debug, sql_schema_describer=info")]
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(
        result.history_problems,
        &[HistoryDiagnostic::MigrationsEdited {
            edited_migration_names: vec![initial_migration_name],
        }]
    );

    Ok(())
}

// TODO: reenable on MySQL when https://github.com/prisma/quaint/issues/187 is fixed.
#[test_each_connector(ignore("mysql"))]
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

    let result = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert_eq!(result.history_problems.len(), 2);

    assert!(result.history_problems.iter().any(|diagnostic| matches!(
        diagnostic,
        HistoryDiagnostic::MigrationsEdited {
            edited_migration_names,
        } if edited_migration_names == &[initial_migration_name.clone()]
    )));

    assert!(result.history_problems.iter().any(|diagnostic| matches!(
        diagnostic,
        HistoryDiagnostic::MigrationFailedToApply {
            migration_name,
            error
        } if migration_name == &initial_migration_name && (error.contains("yolo") || error.contains("YOLO"))
    )));

    Ok(())
}

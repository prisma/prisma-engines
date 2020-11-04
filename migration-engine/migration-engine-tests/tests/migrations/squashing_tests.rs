use crate::*;
use migration_core::commands::{DiagnoseMigrationHistoryOutput, HistoryDiagnostic};
use std::io::Write;

#[test_each_connector]
async fn squashing_whole_migration_history_works(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    // Create and apply a bunch of migrations
    let _initial_migration_names = {
        let dm1 = r#"
            model Cat {
                id Int @id
            }
        "#;

        let dm2 = r#"
            model Cat {
                id Int @id
            }

            model Dog {
                id Int @id
            }
        "#;

        let dm3 = r#"
            model Cat {
                id Int @id
            }

            model Hyena {
                id Int @id
                laughterFrequency Float
            }
        "#;

        let mut migrations_counter: i32 = 0;
        let mut initial_migration_names: Vec<String> = Vec::with_capacity(3);

        for schema in &[dm1, dm2, dm3] {
            let name = api
                .create_migration(&format!("migration{}", migrations_counter), schema, &directory)
                .send()
                .await?
                .into_output()
                .generated_migration_name
                .unwrap();

            migrations_counter += 1;
            initial_migration_names.push(name);
        }

        api.apply_migrations(&directory).send().await?;

        initial_migration_names
    };

    let initial_schema = api.assert_schema().await?.assert_tables_count(3)?.into_schema();

    // Squash the files, mark migration applied, assert the schema is the same.

    let mut squashed_migrations: Vec<(String, String)> = Vec::with_capacity(3);

    for entry in std::fs::read_dir(directory.path())? {
        let entry = entry?;

        assert!(entry.metadata()?.is_dir());

        let file_path = entry.path().join("migration.sql");
        let contents = std::fs::read_to_string(file_path)?;

        squashed_migrations.push((entry.file_name().into_string().unwrap(), contents));

        std::fs::remove_dir_all(entry.path())?;
    }

    squashed_migrations.sort_by(|left, right| left.0.cmp(&right.0));

    let squashed_migration_directory_path = directory.path().join("0000_initial");
    std::fs::create_dir_all(&squashed_migration_directory_path)?;

    let mut migration_file = std::fs::File::create(squashed_migration_directory_path.join("migration.sql"))?;

    for (_, squashed_migration) in squashed_migrations {
        migration_file.write_all(squashed_migration.as_bytes())?;
    }

    api.assert_schema().await?.assert_tables_count(3)?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_some());
    assert!(
        matches!(
            &history,
            Some(HistoryDiagnostic::HistoriesDiverge {
                unapplied_migration_names,
                unpersisted_migration_names: _,
                last_common_migration_name: None,
            }) if unapplied_migration_names == &["0000_initial"]
        ),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.mark_migration_applied("0000_initial", &directory).send().await?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 3),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 3),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.assert_schema().await?.assert_equals(&initial_schema)?;

    // The following does not work because we validate that migrations are failed before marking them as rolled back.
    //
    // // Confirm we can get back to a clean diagnoseMigrationHistory if we mark the squashed migrations as rolled back.
    // {
    //     for migration_name in initial_migration_names {
    //         api.mark_migration_rolled_back(migration_name).send().await?;
    //     }

    //     let DiagnoseMigrationHistoryOutput {
    //         drift,
    //         history,
    //         failed_migration_names,
    //         edited_migration_names,
    //         has_migrations_table,
    //     } = api.diagnose_migration_history(&directory).send().await?.into_output();

    //     assert!(drift.is_none());
    //     assert!(history.is_none());
    //     assert!(failed_migration_names.is_empty());
    //     assert!(edited_migration_names.is_empty());
    //     assert!(has_migrations_table);
    // }

    Ok(())
}

#[test_each_connector(log = "debug")]
async fn squashing_migrations_history_at_the_start_works(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    // Create and apply a bunch of migrations
    let _initial_migration_names = {
        let dm1 = r#"
            model Cat {
                id Int @id
            }
        "#;

        let dm2 = r#"
            model Cat {
                id Int @id
            }

            model Dog {
                id Int @id
            }
        "#;

        let dm3 = r#"
            model Cat {
                id Int @id
            }

            model Hyena {
                id Int @id
                laughterFrequency Float
            }
        "#;

        let mut migrations_counter: i32 = 0;
        let mut initial_migration_names: Vec<String> = Vec::with_capacity(3);

        for schema in &[dm1, dm2, dm3] {
            let name = api
                .create_migration(&format!("migration{}", migrations_counter), schema, &directory)
                .send()
                .await?
                .into_output()
                .generated_migration_name
                .unwrap();

            migrations_counter += 1;
            initial_migration_names.push(name);
        }

        api.apply_migrations(&directory).send().await?;

        initial_migration_names
    };

    let initial_schema = api
        .assert_schema()
        .await?
        .assert_tables_count(3)?
        .assert_has_table("Hyena")?
        .into_schema();

    // Squash the files, mark migration applied, assert the schema is the same.

    let mut squashed_migrations: Vec<(String, String)> = Vec::with_capacity(3);

    for entry in std::fs::read_dir(directory.path())? {
        let entry = entry?;

        assert!(entry.metadata()?.is_dir());

        let file_path = entry.path().join("migration.sql");
        let contents = std::fs::read_to_string(file_path)?;

        squashed_migrations.push((entry.file_name().into_string().unwrap(), contents));
    }

    squashed_migrations.sort_by(|left, right| left.0.cmp(&right.0));

    // Only squash the first two migrations
    squashed_migrations = squashed_migrations.drain(..).take(2).collect();

    for migration_name in squashed_migrations.iter().map(|(a, _)| a) {
        let migration_directory_path = directory.path().join(migration_name);

        tracing::debug!("Deleting migration at {:?}", migration_directory_path);

        std::fs::remove_dir_all(migration_directory_path)?;
    }

    let squashed_migration_directory_path = directory.path().join("0000_initial");
    std::fs::create_dir_all(&squashed_migration_directory_path)?;

    let mut migration_file = std::fs::File::create(squashed_migration_directory_path.join("migration.sql"))?;

    for (_, squashed_migration) in squashed_migrations {
        migration_file.write_all(squashed_migration.as_bytes())?;
    }

    api.assert_schema().await?.assert_tables_count(3)?;
    api.mark_migration_applied("0000_initial", &directory).send().await?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.assert_schema().await?.assert_equals(&initial_schema)?;

    Ok(())
}

#[test_each_connector(log = "debug")]
async fn squashing_migrations_history_at_the_end_works(api: &TestApi) -> TestResult {
    let directory = api.create_migrations_directory()?;

    // Create and apply a bunch of migrations
    let _initial_migration_names = {
        let dm1 = r#"
            model Cat {
                id Int @id
            }
        "#;

        let dm2 = r#"
            model Cat {
                id Int @id
            }

            model Dog {
                id Int @id
            }
        "#;

        let dm3 = r#"
            model Cat {
                id Int @id
            }

            model Hyena {
                id Int @id
                laughterFrequency Float
            }
        "#;

        let mut migrations_counter: i32 = 0;
        let mut initial_migration_names: Vec<String> = Vec::with_capacity(3);

        for schema in &[dm1, dm2, dm3] {
            let name = api
                .create_migration(&format!("migration{}", migrations_counter), schema, &directory)
                .send()
                .await?
                .into_output()
                .generated_migration_name
                .unwrap();

            migrations_counter += 1;
            initial_migration_names.push(name);
        }

        api.apply_migrations(&directory).send().await?;

        initial_migration_names
    };

    let initial_schema = api
        .assert_schema()
        .await?
        .assert_tables_count(3)?
        .assert_has_table("Hyena")?
        .into_schema();

    // Squash the files, mark migration applied, assert the schema is the same.

    let mut squashed_migrations: Vec<(String, String)> = Vec::with_capacity(3);

    for entry in std::fs::read_dir(directory.path())? {
        let entry = entry?;

        assert!(entry.metadata()?.is_dir());

        let file_path = entry.path().join("migration.sql");
        let contents = std::fs::read_to_string(file_path)?;

        squashed_migrations.push((entry.file_name().into_string().unwrap(), contents));
    }

    squashed_migrations.sort_by(|left, right| left.0.cmp(&right.0));

    // Only squash the last two migrations
    squashed_migrations = squashed_migrations.drain(..).skip(1).collect();

    for migration_name in squashed_migrations.iter().map(|(a, _)| a) {
        let migration_directory_path = directory.path().join(migration_name);

        tracing::debug!("Deleting migration at {:?}", migration_directory_path);

        std::fs::remove_dir_all(migration_directory_path)?;
    }

    let squashed_migration_directory_path = directory.path().join("0000_initial");
    std::fs::create_dir_all(&squashed_migration_directory_path)?;

    let mut migration_file = std::fs::File::create(squashed_migration_directory_path.join("migration.sql"))?;

    for (_, squashed_migration) in squashed_migrations {
        migration_file.write_all(squashed_migration.as_bytes())?;
    }

    api.assert_schema().await?.assert_tables_count(3)?;
    api.mark_migration_applied("0000_initial", &directory).send().await?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.apply_migrations(&directory)
        .send()
        .await?
        .assert_applied_migrations(&[])?;

    let DiagnoseMigrationHistoryOutput {
        drift,
        history,
        failed_migration_names,
        edited_migration_names,
        has_migrations_table,
    } = api.diagnose_migration_history(&directory).send().await?.into_output();

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {:#?}",
        history
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.assert_schema().await?.assert_equals(&initial_schema)?;

    Ok(())
}

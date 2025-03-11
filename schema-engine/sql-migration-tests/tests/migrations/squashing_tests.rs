use schema_core::commands::{DiagnoseMigrationHistoryOutput, HistoryDiagnostic};
use sql_migration_tests::test_api::*;
use std::io::Write;

#[test_connector]
fn squashing_whole_migration_history_works(api: TestApi) {
    let directory = api.create_migrations_directory();

    // Create and apply a bunch of migrations
    let _initial_migration_names = {
        let dm1 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }
        "#,
        );

        let dm2 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }

            model Dog {
                id Int @id
            }
        "#,
        );

        let dm3 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }

            model Hyena {
                id Int @id
                laughterFrequency Float
            }
        "#,
        );

        let mut initial_migration_names: Vec<String> = Vec::with_capacity(3);

        for (count, schema) in [dm1, dm2, dm3].iter().enumerate() {
            let name = api
                .create_migration(&format!("migration{count}"), schema, &directory)
                .send_sync()
                .into_output()
                .generated_migration_name;

            initial_migration_names.push(name);
        }

        api.apply_migrations(&directory).send_sync();

        initial_migration_names
    };

    api.assert_schema().assert_tables_count(3);

    // Squash the files, mark migration applied, assert the schema is the same.

    let mut squashed_migrations: Vec<(String, String)> = Vec::with_capacity(3);

    for entry in std::fs::read_dir(directory.path()).unwrap() {
        let entry = entry.unwrap();
        if entry.metadata().unwrap().is_dir() {
            let file_path = entry.path().join("migration.sql");
            let contents = std::fs::read_to_string(file_path).unwrap();

            squashed_migrations.push((entry.file_name().into_string().unwrap(), contents));

            std::fs::remove_dir_all(entry.path()).unwrap();
        }
    }

    squashed_migrations.sort_by(|left, right| left.0.cmp(&right.0));

    let squashed_migration_directory_path = directory.path().join("0000_initial");
    std::fs::create_dir_all(&squashed_migration_directory_path).unwrap();

    let mut migration_file = std::fs::File::create(squashed_migration_directory_path.join("migration.sql")).unwrap();

    for (_, squashed_migration) in squashed_migrations {
        migration_file.write_all(squashed_migration.as_bytes()).unwrap();
    }

    api.assert_schema().assert_tables_count(3);

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
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    api.mark_migration_applied("0000_initial", &directory).send();

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

    assert!(error_in_unapplied_migration.is_none());
    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 3),
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&[]);

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

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 3),
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    // The following does not work because we validate that migrations are failed before marking them as rolled back.
    //
    // // Confirm we can get back to a clean diagnoseMigrationHistory if we mark the squashed migrations as rolled back.
    // {
    //     for migration_name in initial_migration_names {
    //         api.mark_migration_rolled_back(migration_name).send().await.unwrap();
    //     }

    //     let DiagnoseMigrationHistoryOutput {
    //         drift,
    //         history,
    //         failed_migration_names,
    //         edited_migration_names,
    //         has_migrations_table,
    //     } = api.diagnose_migration_history(&directory).send().await.unwrap().into_output();

    //     assert!(drift.is_none());
    //     assert!(history.is_none());
    //     assert!(failed_migration_names.is_empty());
    //     assert!(edited_migration_names.is_empty());
    //     assert!(has_migrations_table);
    // }
}

#[test_connector]
fn squashing_migrations_history_at_the_start_works(api: TestApi) {
    let directory = api.create_migrations_directory();

    // Create and apply a bunch of migrations
    let _initial_migration_names = {
        let dm1 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }
        "#,
        );

        let dm2 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }

            model Dog {
                id Int @id
            }
        "#,
        );

        let dm3 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }

            model Hyena {
                id Int @id
                laughterFrequency Float
            }
        "#,
        );

        let mut initial_migration_names: Vec<String> = Vec::with_capacity(3);

        for (count, schema) in [dm1, dm2, dm3].iter().enumerate() {
            let name = api
                .create_migration(&format!("migration{count}"), schema, &directory)
                .send_sync()
                .into_output()
                .generated_migration_name;

            initial_migration_names.push(name);
        }

        api.apply_migrations(&directory).send_sync();

        initial_migration_names
    };

    api.assert_schema().assert_tables_count(3).assert_has_table("Hyena");

    // Squash the files, mark migration applied, assert the schema is the same.

    let mut squashed_migrations: Vec<(String, String)> = Vec::with_capacity(3);

    for entry in std::fs::read_dir(directory.path()).unwrap() {
        let entry = entry.unwrap();

        if entry.metadata().unwrap().is_dir() {
            let file_path = entry.path().join("migration.sql");
            let contents = std::fs::read_to_string(file_path).unwrap();

            squashed_migrations.push((entry.file_name().into_string().unwrap(), contents));
        }
    }

    squashed_migrations.sort_by(|left, right| left.0.cmp(&right.0));

    // Only squash the first two migrations
    squashed_migrations = squashed_migrations.drain(..).take(2).collect();

    for migration_name in squashed_migrations.iter().map(|(a, _)| a) {
        let migration_directory_path = directory.path().join(migration_name);

        tracing::debug!("Deleting migration at {:?}", migration_directory_path);

        std::fs::remove_dir_all(migration_directory_path).unwrap();
    }

    let squashed_migration_directory_path = directory.path().join("0000_initial");
    std::fs::create_dir_all(&squashed_migration_directory_path).unwrap();

    let mut migration_file = std::fs::File::create(squashed_migration_directory_path.join("migration.sql")).unwrap();

    for (_, squashed_migration) in squashed_migrations {
        migration_file.write_all(squashed_migration.as_bytes()).unwrap();
    }

    api.assert_schema().assert_tables_count(3);
    api.mark_migration_applied("0000_initial", &directory).send();

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

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&[]);

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

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

#[test_connector]
fn squashing_migrations_history_at_the_end_works(api: TestApi) {
    let directory = api.create_migrations_directory();

    // Create and apply a bunch of migrations
    let _initial_migration_names = {
        let dm1 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }
        "#,
        );

        let dm2 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }

            model Dog {
                id Int @id
            }
        "#,
        );

        let dm3 = api.datamodel_with_provider(
            r#"
            model Cat {
                id Int @id
            }

            model Hyena {
                id Int @id
                laughterFrequency Float
            }
        "#,
        );

        let mut initial_migration_names: Vec<String> = Vec::with_capacity(3);

        for (count, schema) in [dm1, dm2, dm3].iter().enumerate() {
            let name = api
                .create_migration(&format!("migration{count}"), schema, &directory)
                .send_sync()
                .into_output()
                .generated_migration_name;

            initial_migration_names.push(name);
        }

        api.apply_migrations(&directory).send_sync();

        initial_migration_names
    };

    api.assert_schema().assert_tables_count(3).assert_has_table("Hyena");

    // Squash the files, mark migration applied, assert the schema is the same.

    let mut squashed_migrations: Vec<(String, String)> = Vec::with_capacity(3);

    for entry in std::fs::read_dir(directory.path()).unwrap() {
        let entry = entry.unwrap();

        if entry.metadata().unwrap().is_dir() {
            let file_path = entry.path().join("migration.sql");
            let contents = std::fs::read_to_string(file_path).unwrap();

            squashed_migrations.push((entry.file_name().into_string().unwrap(), contents));
        }
    }

    squashed_migrations.sort_by(|left, right| left.0.cmp(&right.0));

    // Only squash the last two migrations
    squashed_migrations = squashed_migrations.drain(..).skip(1).collect();

    for migration_name in squashed_migrations.iter().map(|(a, _)| a) {
        let migration_directory_path = directory.path().join(migration_name);

        tracing::debug!("Deleting migration at {:?}", migration_directory_path);

        std::fs::remove_dir_all(migration_directory_path).unwrap();
    }

    let squashed_migration_directory_path = directory.path().join("0000_initial");
    std::fs::create_dir_all(&squashed_migration_directory_path).unwrap();

    let mut migration_file = std::fs::File::create(squashed_migration_directory_path.join("migration.sql")).unwrap();

    for (_, squashed_migration) in squashed_migrations {
        migration_file.write_all(squashed_migration.as_bytes()).unwrap();
    }

    api.assert_schema().assert_tables_count(3);
    api.mark_migration_applied("0000_initial", &directory).send();

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

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());

    api.apply_migrations(&directory)
        .send_sync()
        .assert_applied_migrations(&[]);

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

    assert!(drift.is_none());
    assert!(
        matches!(&history, Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names }) if unpersisted_migration_names.len() == 2),
        "got: {history:#?}"
    );
    assert!(failed_migration_names.is_empty());
    assert!(edited_migration_names.is_empty());
    assert!(has_migrations_table);
    assert!(error_in_unapplied_migration.is_none());
}

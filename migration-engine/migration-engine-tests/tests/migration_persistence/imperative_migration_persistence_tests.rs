use chrono::Duration;
use migration_engine_tests::*;
use pretty_assertions::assert_eq;

#[test_each_connector]
async fn starting_a_migration_works(api: &TestApi) -> TestResult {
    let persistence = api.imperative_migration_persistence();

    let script = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id = persistence
        .record_migration_started("initial_migration", script)
        .await?;

    let migrations = persistence.list_migrations().await?;

    assert_eq!(migrations.len(), 1);

    let first_migration = &migrations[0];

    assert_eq!(first_migration.id, id);
    assert_eq!(
        first_migration.checksum,
        "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec72e772429fb20d8cc69a864"
    );
    assert_eq!(first_migration.finished_at, None);
    assert_eq!(first_migration.migration_name, "initial_migration");
    assert_eq!(first_migration.logs, "");
    assert_eq!(first_migration.rolled_back_at, None);
    assert_eq!(first_migration.applied_steps_count, 0);
    assert_eq!(first_migration.script, script);

    let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);

    assert!(duration_since_started_at >= Duration::seconds(0));
    assert!(duration_since_started_at < Duration::seconds(1));

    Ok(())
}

#[test_each_connector]
async fn finishing_a_migration_works(api: &TestApi) -> TestResult {
    let persistence = api.imperative_migration_persistence();

    let script = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id = persistence
        .record_migration_started("initial_migration", script)
        .await?;
    persistence.record_migration_finished(&id).await?;

    let migrations = persistence.list_migrations().await?;

    assert_eq!(migrations.len(), 1);

    let first_migration = &migrations[0];

    assert_eq!(first_migration.id, id);
    assert_eq!(
        first_migration.checksum,
        "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec72e772429fb20d8cc69a864"
    );
    assert_eq!(first_migration.migration_name, "initial_migration");
    assert_eq!(first_migration.logs, "");
    assert_eq!(first_migration.rolled_back_at, None);
    assert_eq!(first_migration.applied_steps_count, 0);
    assert_eq!(first_migration.script, script);

    let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);
    let duration_between_started_at_and_finished_at =
        chrono::Utc::now().signed_duration_since(first_migration.started_at);

    assert!(duration_since_started_at >= Duration::seconds(0));
    assert!(duration_since_started_at < Duration::seconds(10));
    assert!(duration_between_started_at_and_finished_at >= Duration::seconds(0));
    assert!(duration_between_started_at_and_finished_at < Duration::seconds(10));

    Ok(())
}

#[test_each_connector]
async fn updating_then_finishing_a_migration_works(api: &TestApi) -> TestResult {
    let persistence = api.imperative_migration_persistence();

    let script = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id = persistence
        .record_migration_started("initial_migration", script)
        .await?;
    persistence.record_successful_step(&id, "oï").await?;
    persistence.record_migration_finished(&id).await?;

    let migrations = persistence.list_migrations().await?;

    assert_eq!(migrations.len(), 1);

    let first_migration = &migrations[0];

    assert_eq!(first_migration.id, id);
    assert_eq!(
        first_migration.checksum,
        "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec72e772429fb20d8cc69a864"
    );
    assert_eq!(first_migration.migration_name, "initial_migration");
    assert_eq!(first_migration.logs, "oï");
    assert_eq!(first_migration.rolled_back_at, None);
    assert_eq!(first_migration.applied_steps_count, 1);
    assert_eq!(first_migration.script, script);

    let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);
    let duration_between_started_at_and_finished_at =
        chrono::Utc::now().signed_duration_since(first_migration.started_at);

    assert!(duration_since_started_at >= Duration::seconds(0));
    assert!(duration_since_started_at < Duration::seconds(10));
    assert!(duration_between_started_at_and_finished_at >= Duration::seconds(0));
    assert!(duration_between_started_at_and_finished_at < Duration::seconds(10));

    Ok(())
}

#[test_each_connector]
async fn multiple_successive_migrations_work(api: &TestApi) -> TestResult {
    let persistence = api.imperative_migration_persistence();

    let script_1 = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id_1 = persistence
        .record_migration_started("initial_migration", script_1)
        .await?;
    persistence.record_successful_step(&id_1, "oï").await?;
    persistence.record_migration_finished(&id_1).await?;

    std::thread::sleep(std::time::Duration::from_millis(10));

    let script_2 = "DROP ENUM MyBoolean";
    let id_2 = persistence
        .record_migration_started("second_migration", script_2)
        .await?;
    persistence
        .record_successful_step(&id_2, "logs for the second migration")
        .await?;

    let migrations = persistence.list_migrations().await?;

    assert_eq!(migrations.len(), 2);

    // First migration assertions
    {
        let first_migration = &migrations[0];

        assert_eq!(first_migration.id, id_1);
        assert_eq!(
            first_migration.checksum,
            "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec72e772429fb20d8cc69a864"
        );
        assert_eq!(first_migration.migration_name, "initial_migration");
        assert_eq!(first_migration.logs, "oï");
        assert_eq!(first_migration.rolled_back_at, None);
        assert_eq!(first_migration.applied_steps_count, 1);
        assert_eq!(first_migration.script, script_1);

        let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);
        let duration_between_started_at_and_finished_at =
            chrono::Utc::now().signed_duration_since(first_migration.started_at);

        assert!(duration_since_started_at >= Duration::seconds(0));
        assert!(duration_since_started_at < Duration::seconds(10));
        assert!(duration_between_started_at_and_finished_at >= Duration::seconds(0));
        assert!(duration_between_started_at_and_finished_at < Duration::seconds(10));
    }

    // Second migration assertions
    {
        let second_migration = &migrations[1];

        assert_eq!(second_migration.id, id_2);
        assert_eq!(
            second_migration.checksum,
            "822db1ee793d76eaa1319eb2c453a7ec92ab6ec235268b4d27ac395c6c5a6ef"
        );
        assert_eq!(second_migration.migration_name, "second_migration");
        assert_eq!(second_migration.logs, "logs for the second migration");
        assert_eq!(second_migration.rolled_back_at, None);
        assert_eq!(second_migration.applied_steps_count, 1);
        assert_eq!(second_migration.script, script_2);
        assert_eq!(second_migration.finished_at, None);

        let duration_since_started_at = chrono::Utc::now().signed_duration_since(second_migration.started_at);

        assert!(duration_since_started_at >= Duration::seconds(0));
        assert!(duration_since_started_at < Duration::seconds(10));
    }

    Ok(())
}

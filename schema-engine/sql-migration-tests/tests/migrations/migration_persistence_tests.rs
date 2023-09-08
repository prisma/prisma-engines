use chrono::Duration;
use pretty_assertions::assert_eq;
use sql_migration_tests::test_api::*;

#[test_connector]
fn starting_a_migration_works(api: TestApi) {
    let persistence = api.migration_persistence();

    tok(persistence.initialize(None)).unwrap();

    let script = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id = tok(persistence.record_migration_started("initial_migration", script)).unwrap();

    let migrations = tok(persistence.list_migrations()).unwrap().unwrap();

    assert_eq!(migrations.len(), 1);

    let first_migration = &migrations[0];

    assert_eq!(first_migration.id, id);
    assert_eq!(
        first_migration.checksum,
        "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec720e07072429fb20d8cc69a864"
    );
    assert_eq!(first_migration.finished_at, None);
    assert_eq!(first_migration.migration_name, "initial_migration");
    assert_eq!(first_migration.logs.as_deref(), None);
    assert_eq!(first_migration.rolled_back_at, None);
    assert_eq!(first_migration.applied_steps_count, 0);

    let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);

    assert!(duration_since_started_at >= Duration::seconds(0));
    assert!(duration_since_started_at < Duration::seconds(1));
}

#[test_connector]
fn finishing_a_migration_works(api: TestApi) {
    let persistence = api.migration_persistence();

    tok(persistence.initialize(None)).unwrap();

    let script = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id = tok(persistence.record_migration_started("initial_migration", script)).unwrap();
    tok(persistence.record_migration_finished(&id)).unwrap();

    let migrations = tok(persistence.list_migrations()).unwrap().unwrap();

    assert_eq!(migrations.len(), 1);

    let first_migration = &migrations[0];

    assert_eq!(first_migration.id, id);
    assert_eq!(
        first_migration.checksum,
        "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec720e07072429fb20d8cc69a864"
    );
    assert_eq!(first_migration.migration_name, "initial_migration");
    assert_eq!(first_migration.logs.as_deref(), None);
    assert_eq!(first_migration.rolled_back_at, None);
    assert_eq!(first_migration.applied_steps_count, 0);

    let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);
    let duration_between_started_at_and_finished_at =
        chrono::Utc::now().signed_duration_since(first_migration.started_at);

    assert!(duration_since_started_at >= Duration::seconds(0));
    assert!(duration_since_started_at < Duration::seconds(10));
    assert!(duration_between_started_at_and_finished_at >= Duration::seconds(0));
    assert!(duration_between_started_at_and_finished_at < Duration::seconds(10));
}

#[test_connector]
fn updating_then_finishing_a_migration_works(api: TestApi) {
    let persistence = api.migration_persistence();

    tok(persistence.initialize(None)).unwrap();

    let script = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id = tok(persistence.record_migration_started("initial_migration", script)).unwrap();
    tok(persistence.record_successful_step(&id)).unwrap();
    tok(persistence.record_migration_finished(&id)).unwrap();

    let migrations = tok(persistence.list_migrations()).unwrap().unwrap();

    assert_eq!(migrations.len(), 1);

    let first_migration = &migrations[0];

    assert_eq!(first_migration.id, id);
    assert_eq!(
        first_migration.checksum,
        "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec720e07072429fb20d8cc69a864"
    );
    assert_eq!(first_migration.migration_name, "initial_migration");
    assert!(first_migration.logs.is_none());
    assert_eq!(first_migration.rolled_back_at, None);
    assert_eq!(first_migration.applied_steps_count, 1);

    let duration_since_started_at = chrono::Utc::now().signed_duration_since(first_migration.started_at);
    let duration_between_started_at_and_finished_at =
        chrono::Utc::now().signed_duration_since(first_migration.started_at);

    assert!(duration_since_started_at >= Duration::seconds(0));
    assert!(duration_since_started_at < Duration::seconds(10));
    assert!(duration_between_started_at_and_finished_at >= Duration::seconds(0));
    assert!(duration_between_started_at_and_finished_at < Duration::seconds(10));
}

#[test_connector]
fn multiple_successive_migrations_work(api: TestApi) {
    let persistence = api.migration_persistence();

    tok(persistence.initialize(None)).unwrap();

    let script_1 = "CREATE ENUM MyBoolean ( \"TRUE\", \"FALSE\" )";

    let id_1 = tok(persistence.record_migration_started("initial_migration", script_1)).unwrap();
    tok(persistence.record_successful_step(&id_1)).unwrap();
    tok(persistence.record_migration_finished(&id_1)).unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let script_2 = "DROP ENUM MyBoolean";
    let id_2 = tok(persistence.record_migration_started("second_migration", script_2)).unwrap();
    tok(persistence.record_successful_step(&id_2)).unwrap();

    let migrations = tok(persistence.list_migrations()).unwrap().unwrap();

    assert_eq!(migrations.len(), 2);

    // First migration assertions
    {
        let first_migration = &migrations[0];

        assert_eq!(first_migration.id, id_1);
        assert_eq!(
            first_migration.checksum,
            "e0c9674d3b332d71b8bc304aae5b7b8a8bb8ec720e07072429fb20d8cc69a864"
        );
        assert_eq!(first_migration.migration_name, "initial_migration");
        assert!(first_migration.logs.is_none());
        assert_eq!(first_migration.rolled_back_at, None);
        assert_eq!(first_migration.applied_steps_count, 1);

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
            "822db1ee793d76eaa1319eb2c453a7ec92ab6ec235268b4d27ac395c6c5a6e0f"
        );
        assert_eq!(second_migration.migration_name, "second_migration");
        assert!(second_migration.logs.is_none());
        assert_eq!(second_migration.rolled_back_at, None);
        assert_eq!(second_migration.applied_steps_count, 1);
        assert_eq!(second_migration.finished_at, None);

        let duration_since_started_at = chrono::Utc::now().signed_duration_since(second_migration.started_at);

        assert!(duration_since_started_at >= Duration::seconds(0));
        assert!(duration_since_started_at < Duration::seconds(10));
    }
}

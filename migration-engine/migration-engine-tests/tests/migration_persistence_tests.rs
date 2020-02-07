use migration_connector::{steps::CreateEnum, *};
use migration_engine_tests::*;
use pretty_assertions::assert_eq;
use quaint::prelude::SqlFamily;

#[test_each_connector]
async fn last_should_return_none_if_there_is_no_migration(api: &TestApi) {
    let persistence = api.migration_persistence();
    let result = persistence.last().await.unwrap();
    assert_eq!(result.is_some(), false);
}

#[test_each_connector]
async fn last_must_return_none_if_there_is_no_successful_migration(api: &TestApi) {
    let persistence = api.migration_persistence();
    persistence.create(Migration::new("my_migration".to_string()));
    let loaded = persistence.last().await.unwrap();
    assert_eq!(loaded, None);
}

#[test_each_connector]
async fn load_all_should_return_empty_if_there_is_no_migration(api: &TestApi) {
    let persistence = api.migration_persistence();
    let result = persistence.load_all().await.unwrap();
    assert_eq!(result.is_empty(), true);
}

#[test_each_connector]
async fn load_all_must_return_all_created_migrations(api: &TestApi) {
    let persistence = api.migration_persistence();
    let migration1 = persistence
        .create(Migration::new("migration_1".to_string()))
        .await
        .unwrap();
    let migration2 = persistence
        .create(Migration::new("migration_2".to_string()))
        .await
        .unwrap();
    let migration3 = persistence
        .create(Migration::new("migration_3".to_string()))
        .await
        .unwrap();

    let mut result = persistence.load_all().await.unwrap();
    if api.sql_family() == SqlFamily::Mysql {
        // TODO: mysql currently looses milli seconds on loading
        result[0].started_at = migration1.started_at;
        result[1].started_at = migration2.started_at;
        result[2].started_at = migration3.started_at;
    }
    assert_eq!(result, vec![migration1, migration2, migration3])
}

#[test_each_connector]
async fn create_should_allow_to_create_a_new_migration(api: &TestApi) {
    let dm = r#"
        model Test {
            id String @id @default(cuid())
        }
    "#;

    let persistence = api.migration_persistence();
    let mut migration = Migration::new("my_migration".to_string());
    migration.status = MigrationStatus::MigrationSuccess;
    migration.datamodel_string = dm.to_owned();
    migration.datamodel_steps = vec![MigrationStep::CreateEnum(CreateEnum {
        r#enum: "MyEnum".to_string(),
        values: vec!["A".to_string(), "B".to_string()],
    })];
    migration.errors = vec!["error1".to_string(), "error2".to_string()];

    let result = persistence.create(migration.clone()).await.unwrap();
    migration.revision = result.revision; // copy over the generated revision so that the assertion can work.`

    assert_eq!(result, migration);
    let mut loaded = persistence.last().await.unwrap().unwrap();

    if api.sql_family() == SqlFamily::Mysql {
        // TODO: mysql currently looses milli seconds on loading
        loaded.started_at = migration.started_at;
    }

    assert_eq!(loaded, migration);
}

#[test_each_connector]
async fn create_should_increment_revisions(api: &TestApi) {
    let persistence = api.migration_persistence();
    let migration1 = persistence
        .create(Migration::new("migration_1".to_string()))
        .await
        .unwrap();
    let migration2 = persistence
        .create(Migration::new("migration_2".to_string()))
        .await
        .unwrap();
    assert_eq!(migration1.revision + 1, migration2.revision);
}

#[test_each_connector]
async fn update_must_work(api: &TestApi) {
    let persistence = api.migration_persistence();
    let migration = persistence
        .create(Migration::new("my_migration".to_string()))
        .await
        .unwrap();

    let mut params = migration.update_params();
    params.status = MigrationStatus::MigrationSuccess;
    params.applied = 10;
    params.rolled_back = 11;
    params.errors = vec!["err1".to_string(), "err2".to_string()];
    params.finished_at = Some(Migration::timestamp_without_nanos());
    params.new_name = "my_new_migration_name".to_string();

    persistence.update(&params).await.unwrap();

    let loaded = persistence.last().await.unwrap().unwrap();
    assert_eq!(loaded.status, params.status);
    assert_eq!(loaded.applied, params.applied);
    assert_eq!(loaded.rolled_back, params.rolled_back);
    assert_eq!(loaded.errors, params.errors);
    if api.sql_family() != SqlFamily::Mysql {
        // TODO: mysql currently looses milli seconds on loading
        assert_eq!(loaded.finished_at, params.finished_at);
    }
    assert_eq!(loaded.name, params.new_name);
}

#[test_each_connector]
async fn migration_is_already_applied_must_work(api: &TestApi) -> TestResult {
    let persistence = api.migration_persistence();

    let mut migration_1 = Migration::new("migration_1".to_string());
    migration_1.status = MigrationStatus::MigrationSuccess;

    persistence.create(migration_1).await?;

    let mut migration_2 = Migration::new("migration_2".to_string());
    migration_2.status = MigrationStatus::MigrationFailure;

    persistence.create(migration_2).await?;

    assert!(persistence.migration_is_already_applied("migration_1").await?);
    assert!(!persistence.migration_is_already_applied("migration_2").await?);
    assert!(!persistence.migration_is_already_applied("another_migration").await?);

    Ok(())
}

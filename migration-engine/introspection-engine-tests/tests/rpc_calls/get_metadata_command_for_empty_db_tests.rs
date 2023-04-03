use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use pretty_assertions::assert_eq;
use test_macros::test_connector;

async fn setup_empty(barrel: &BarrelMigrationExecutor, db_name: &str) -> TestResult {
    barrel.execute_with_schema(|_| {}, db_name).await?;

    Ok(())
}

#[test_connector]
async fn empty_metadata_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;

    let metadata = api.get_metadata().await?;

    assert_eq!(0, metadata.table_count);
    assert_eq!(0, metadata.size_in_bytes);

    Ok(())
}

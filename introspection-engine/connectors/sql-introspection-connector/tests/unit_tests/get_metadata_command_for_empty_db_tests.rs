use crate::test_harness::*;
use crate::{test_each_connector,BarrelMigrationExecutor, TestApi};

#[test_each_connector]
async fn empty_metadata_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup_empty(&barrel, api.schema_name()).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 0);
    assert_eq!(result.size_in_bytes, 0);
}

async fn setup_empty(barrel: &BarrelMigrationExecutor, db_name: &str) {
    let _setup_schema = barrel
        .execute_with_schema(|_| {}, db_name)
        .await;
}
use crate::test_harness::*;
use crate::{test_one_connector,BarrelMigrationExecutor, TestApi};

#[test_one_connector(connector = "mysql")]
async fn empty_metadata_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup_empty(&barrel).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 0);
}

async fn setup_empty(barrel: &BarrelMigrationExecutor) {
    let _setup_schema = barrel
        .execute(|migration| {
            migration.drop_table_if_exists("test")

        })
        .await;
}
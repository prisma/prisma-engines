use crate::test_harness::{*};
use crate::{test_one_connector, BarrelMigrationExecutor, TestApi};
use barrel::types;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test_one_connector(connector = "mysql")]
async fn metadata_for_mysql_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 49152);
}

#[test_one_connector(connector = "postgres")]

async fn metadata_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 40960);
}

#[test_one_connector(connector = "sqlite")]
async fn metadata_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 0); // page_size * page_count and count is 0
}

async fn setup(barrel: &BarrelMigrationExecutor) {
    let _setup_schema = barrel.execute(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("bool", types::boolean());
            t.add_column("float", types::float());
            t.add_column("date", types::date());
            t.add_column("id", types::primary());
            t.add_column("int", types::integer());
            t.add_column("string", types::text());
        });

        migration.create_table("Blog2", |t| {
            t.add_column("id", types::primary());
            t.add_column("int", types::integer());
            t.add_column("string", types::text());
        });

        migration.create_table("Blog3", |t| {
            t.add_column("bool", types::boolean());
            t.add_column("float", types::float());
            t.add_column("date", types::date());
            t.add_column("id", types::primary());
        });
    }).await;
}

use crate::test_harness::*;
use crate::{BarrelMigrationExecutor, TestApi};
use barrel::types;

#[test_each_connector(tags("mysql"))]
async fn metadata_for_mysql_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.db_name()).await;
    let result = api.get_metadata().await;
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 49152);
}

#[test_each_connector(tags("postgres"))]
async fn metadata_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.schema_name()).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 40960);
}

#[test_each_connector(tags("sqlite"))]
async fn metadata_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.schema_name()).await;
    let result = dbg!(api.get_metadata().await);
    assert_eq!(result.table_count, 3);
    assert_eq!(result.size_in_bytes, 0); // page_size * page_count and count is 0
}

async fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) {
    let _setup_schema = barrel
        .execute_with_schema(
            |migration| {
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
            },
            db_name,
        )
        .await;
}

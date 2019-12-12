use crate::{test_harness::*, test_one_connector, BarrelMigrationExecutor};
use barrel::types;

#[test_one_connector(connector = "mysql")]
async fn databases_for_mysql_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.db_name());
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&api.db_name().to_string()));
}

#[test_one_connector(connector = "mysql_8")]
async fn databases_for_mysql_8_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.db_name());
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&api.db_name().to_string()));
}

#[test_one_connector(connector = "postgres")]
async fn databases_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, SCHEMA_NAME);
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&SCHEMA_NAME.to_string()));
}

#[test_one_connector(connector = "sqlite")]
async fn databases_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, SCHEMA_NAME);
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&format!("{}.db", "databases_for_sqlite_should_work")));
}

fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) {
    let _setup_schema = barrel.execute_with_schema(|migration| {
        migration.create_table("Blog", |t| {
            t.add_column("id", types::primary());
        });
    }, db_name);
}

use crate::{test_harness::*, test_one_connector, BarrelMigrationExecutor};
use barrel::types;

pub const SCHEMA_NAME: &str = "introspection-engine";

#[test_one_connector(connector = "mysql")]
async fn databases_for_mysql_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel);
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&"introspection-engine".to_string()));
}

#[test_one_connector(connector = "postgres")]
async fn databases_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel);
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&"introspection-engine".to_string()));
}

#[test_one_connector(connector = "sqlite")]
async fn databases_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel);
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&"introspection-engine.db".to_string()));
}

fn setup(barrel: &BarrelMigrationExecutor) {
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
    });
}

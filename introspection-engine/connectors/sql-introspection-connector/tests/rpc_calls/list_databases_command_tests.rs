use crate::{test_harness::*, BarrelMigrationExecutor};
use barrel::types;

#[test_each_connector(tags("mysql"))]
async fn databases_for_mysql_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.db_name());
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&api.db_name().to_string()));
}

#[test_each_connector(tags("postgres"))]
async fn databases_for_postgres_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.schema_name());
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&api.schema_name().to_string()));
}

#[test_each_connector(tags("sqlite"))]
async fn databases_for_sqlite_should_work(api: &TestApi) {
    let barrel = api.barrel();
    setup(&barrel, api.schema_name());
    let result = dbg!(api.list_databases().await);
    assert!(result.contains(&format!("{}.db", "databases_for_sqlite_should_work")));
}

fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) {
    let _setup_schema = barrel.execute_with_schema(
        |migration| {
            migration.create_table("Blog", |t| {
                t.add_column("id", types::primary());
            });
        },
        db_name,
    );
}

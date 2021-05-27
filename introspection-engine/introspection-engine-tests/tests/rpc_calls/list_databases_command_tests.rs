use barrel::types;
use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use test_macros::test_connector;

#[test_connector(tags(Mysql), exclude(Vitess))]
async fn databases_for_mysql_should_work(api: &TestApi) -> TestResult {
    setup(&api.barrel(), api.db_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.contains(&api.db_name().to_string()));

    Ok(())
}

#[test_connector(tags(Postgres))]
async fn databases_for_postgres_should_work(api: &TestApi) -> TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.contains(&api.schema_name().to_string()));

    Ok(())
}

#[test_connector(tags(Mssql))]
async fn databases_for_mssql_should_work(api: &TestApi) -> TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.contains(&api.schema_name().to_string()));

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn databases_for_sqlite_should_work(api: &TestApi) -> TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.iter().any(|db| db == "databases_for_sqlite_should_work"));

    Ok(())
}

async fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) -> TestResult {
    barrel
        .execute_with_schema(
            |migration| {
                migration.create_table("Blog", |t| {
                    t.add_column("id", types::primary());
                });
            },
            db_name,
        )
        .await?;

    Ok(())
}

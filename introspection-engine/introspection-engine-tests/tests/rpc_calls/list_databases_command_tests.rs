use barrel::types;
use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use test_macros::test_each_connector_mssql as test_each_connector;

#[test_each_connector(tags("mysql"))]
async fn databases_for_mysql_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.db_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.contains(&api.db_name().to_string()));

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn databases_for_postgres_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.contains(&api.schema_name().to_string()));

    Ok(())
}

#[test_each_connector(tags("mssql_2017", "mssql_2019"))]
async fn databases_for_mssql_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.contains(&api.schema_name().to_string()));

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn databases_for_sqlite_should_work(api: &TestApi) -> crate::TestResult {
    setup(&api.barrel(), api.schema_name()).await?;

    let result = api.list_databases().await?;
    assert!(result.iter().any(|db| db == "databases_for_sqlite_should_work"));

    Ok(())
}

async fn setup(barrel: &BarrelMigrationExecutor, db_name: &str) -> crate::TestResult {
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

use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use test_macros::test_connector;

async fn setup_empty(barrel: &BarrelMigrationExecutor, db_name: &str) -> TestResult {
    barrel.execute_with_schema(|_| {}, db_name).await?;

    Ok(())
}

#[test_connector(tags(Sqlite))]
async fn database_version_for_sqlite_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("3.35.4"));

    Ok(())
}

#[test_connector(tags(Mysql56))]
async fn database_version_for_mysql_5_6_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("5.6"));

    Ok(())
}

#[test_connector(tags(Mariadb))]
async fn database_version_for_mariadb_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("MariaDB"));

    Ok(())
}

#[test_connector(tags(Mysql8))]
async fn database_version_for_mysql_8_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("8.0"));

    Ok(())
}

#[test_connector(tags(Postgres), exclude(CockroachDb))]
async fn database_version_for_postgres_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("PostgreSQL"));

    Ok(())
}

#[test_connector(tags(CockroachDb))]
async fn database_version_for_cockroach_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("CockroachDB"));

    Ok(())
}

#[test_connector(tags(Mssql2017))]
async fn database_version_for_mssql_2017_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("Microsoft SQL Server 2017"));

    Ok(())
}

#[test_connector(tags(Mssql2019))]
async fn database_version_for_mssql_2019_should_work(api: &TestApi) -> TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert!(result.contains("Microsoft SQL Server 2019"));

    Ok(())
}

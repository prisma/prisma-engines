use introspection_engine_tests::{test_api::*, BarrelMigrationExecutor};
use pretty_assertions::assert_eq;
use test_macros::test_each_connector_mssql as test_each_connector;

async fn setup_empty(barrel: &BarrelMigrationExecutor, db_name: &str) -> crate::TestResult {
    barrel.execute_with_schema(|_| {}, db_name).await?;

    Ok(())
}

#[test_each_connector(tags("sqlite"))]
async fn database_version_for_sqlite_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("3.31"));

    Ok(())
}

#[test_each_connector(tags("mysql_5_6"))]
async fn database_version_for_mysql_5_6_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("5.6"));

    Ok(())
}

#[test_each_connector(tags("mariadb"))]
async fn database_version_for_mariadb_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("MariaDB"));

    Ok(())
}

#[test_each_connector(tags("mysql_8"))]
async fn database_version_for_mysql_8_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("8.0"));

    Ok(())
}

#[test_each_connector(tags("postgres"))]
async fn database_version_for_postgres_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("PostgreSQL"));

    Ok(())
}

#[test_each_connector(tags("mssql_2017"))]
async fn database_version_for_mssql_2017_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("Microsoft SQL Server 2017"));

    Ok(())
}

#[test_each_connector(tags("mssql_2019"))]
async fn database_version_for_mssql_2019_should_work(api: &TestApi) -> crate::TestResult {
    setup_empty(&api.barrel(), api.schema_name()).await?;
    let result = api.get_database_version().await?;
    assert_eq!(true, result.contains("Microsoft SQL Server 2019"));

    Ok(())
}

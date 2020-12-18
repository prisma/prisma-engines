use sql_migration_connector::SqlMigrationConnector;
use test_setup::*;

pub type TestResult = Result<(), anyhow::Error>;

pub(super) async fn mysql_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_mysql_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str).await.unwrap()
}

pub(super) async fn postgres_migration_connector(url_str: &str) -> SqlMigrationConnector {
    create_postgres_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str).await.unwrap()
}

pub(super) async fn sqlite_migration_connector(db_name: &str) -> SqlMigrationConnector {
    let database_url = sqlite_test_url(db_name);
    SqlMigrationConnector::new(&database_url).await.unwrap()
}

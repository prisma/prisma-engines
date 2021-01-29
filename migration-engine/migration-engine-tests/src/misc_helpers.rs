use enumflags2::BitFlags;
use migration_connector::*;
use sql_migration_connector::{SqlMigrationConnector, SqlMigrationConnectorParams};
use test_setup::*;

pub type TestResult = Result<(), anyhow::Error>;

pub(super) async fn mysql_migration_connector(
    url_str: &str,
    features: BitFlags<MigrationFeature>,
) -> SqlMigrationConnector {
    create_mysql_database(&url_str.parse().unwrap()).await.unwrap();

    let params = SqlMigrationConnectorParams {
        datasource_url: url_str,
        features,
        datasource_shadow_database_url: None,
    };

    SqlMigrationConnector::new(params).await.unwrap()
}

pub(super) async fn postgres_migration_connector(
    url_str: &str,
    features: BitFlags<MigrationFeature>,
) -> SqlMigrationConnector {
    create_postgres_database(&url_str.parse().unwrap()).await.unwrap();

    let params = SqlMigrationConnectorParams {
        datasource_url: url_str,
        features,
        datasource_shadow_database_url: None,
    };

    SqlMigrationConnector::new(params).await.unwrap()
}

pub(super) async fn sqlite_migration_connector(
    db_name: &str,
    features: BitFlags<MigrationFeature>,
) -> SqlMigrationConnector {
    let database_url = sqlite_test_url(db_name);

    let params = SqlMigrationConnectorParams {
        datasource_url: &database_url,
        features,
        datasource_shadow_database_url: None,
    };

    SqlMigrationConnector::new(params).await.unwrap()
}

use datamodel::ast::{parser, SchemaAst};
use enumflags2::BitFlags;
use migration_connector::*;
use migration_core::api::MigrationApi;
use sql_migration_connector::SqlMigrationConnector;
use test_setup::*;

pub type TestResult = Result<(), anyhow::Error>;

pub fn parse(datamodel_string: &str) -> SchemaAst {
    parser::parse_schema(datamodel_string).unwrap()
}

pub(super) async fn mysql_migration_connector(
    url_str: &str,
    features: BitFlags<MigrationFeature>,
) -> SqlMigrationConnector {
    create_mysql_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str, features).await.unwrap()
}

pub(super) async fn postgres_migration_connector(
    url_str: &str,
    features: BitFlags<MigrationFeature>,
) -> SqlMigrationConnector {
    create_postgres_database(&url_str.parse().unwrap()).await.unwrap();
    SqlMigrationConnector::new(url_str, features).await.unwrap()
}

pub(super) async fn sqlite_migration_connector(
    db_name: &str,
    features: BitFlags<MigrationFeature>,
) -> SqlMigrationConnector {
    let database_url = sqlite_test_url(db_name);

    SqlMigrationConnector::new(&database_url, features).await.unwrap()
}

pub async fn test_api<C, D>(connector: C) -> MigrationApi<C, D>
where
    C: MigrationConnector<DatabaseMigration = D>,
    D: DatabaseMigrationMarker + Send + Sync + 'static,
{
    MigrationApi::new(connector).await.unwrap()
}

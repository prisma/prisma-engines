mod mssql_datamodel_connector;
mod mysql_datamodel_connector;
mod postgres_datamodel_connector;
mod sqlite_datamodel_connector;

pub use mssql_datamodel_connector::MsSqlDatamodelConnector;
pub use mysql_datamodel_connector::MySqlDatamodelConnector;
pub use postgres_datamodel_connector::PostgresDatamodelConnector;
pub use sqlite_datamodel_connector::SqliteDatamodelConnector;

use datamodel_connector::ReferentialIntegrity;

pub struct SqlDatamodelConnectors {}

impl SqlDatamodelConnectors {
    pub fn postgres() -> PostgresDatamodelConnector {
        PostgresDatamodelConnector::new()
    }

    pub fn mysql(referential_integrity: ReferentialIntegrity) -> MySqlDatamodelConnector {
        MySqlDatamodelConnector::new(referential_integrity)
    }

    pub fn sqlite() -> SqliteDatamodelConnector {
        SqliteDatamodelConnector::new()
    }

    pub fn mssql() -> MsSqlDatamodelConnector {
        MsSqlDatamodelConnector::new()
    }
}

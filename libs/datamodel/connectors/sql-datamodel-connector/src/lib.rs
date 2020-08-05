mod mssql_datamodel_connector;
mod mysql_datamodel_connector;
mod postgres_datamodel_connector;
mod sqlite_datamodel_connector;

use datamodel_connector::DeclarativeConnector;

pub struct SqlDatamodelConnectors {}

impl SqlDatamodelConnectors {
    pub fn postgres() -> DeclarativeConnector {
        postgres_datamodel_connector::new()
    }

    pub fn mysql() -> DeclarativeConnector {
        mysql_datamodel_connector::new()
    }

    pub fn sqlite() -> DeclarativeConnector {
        sqlite_datamodel_connector::new()
    }

    pub fn mssql() -> DeclarativeConnector {
        mssql_datamodel_connector::new()
    }
}

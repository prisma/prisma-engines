use crate::{connection_wrapper::Connection, flavour::SqlFlavour, DatabaseInfo, SqlMigrationConnector};
use migration_connector::ConnectorResult;
use quaint::prelude::{ConnectionInfo, SqlFamily};
use sql_schema_describer::SqlSchema;

/// Implemented by the components of the connector that contain a reference to the connector (like
/// SqlDestructiveChangeChecker). It lets them conveniently access global resources.
#[async_trait::async_trait]
pub(crate) trait Component {
    fn connector(&self) -> &SqlMigrationConnector;

    fn schema_name(&self) -> &str {
        &self.connection_info().schema_name()
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.connector().database_info.connection_info()
    }

    fn conn(&self) -> Connection<'_> {
        Connection::new(&self.connector().database)
    }

    fn database_info(&self) -> &DatabaseInfo {
        &self.connector().database_info
    }

    async fn describe(&self) -> ConnectorResult<SqlSchema> {
        self.connector().describe_schema().await
    }

    fn sql_family(&self) -> SqlFamily {
        self.connection_info().sql_family()
    }

    fn flavour(&self) -> &(dyn SqlFlavour + Send + Sync + 'static) {
        self.connector().flavour.as_ref()
    }
}

#[async_trait::async_trait]
impl Component for SqlMigrationConnector {
    fn connector(&self) -> &SqlMigrationConnector {
        self
    }
}

use crate::{DatabaseInfo, SqlMigrationConnector, SqlResult};
use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};
use sql_schema_describer::SqlSchema;

#[async_trait::async_trait]
pub(crate) trait Component {
    fn connector(&self) -> &SqlMigrationConnector;

    fn schema_name(&self) -> &str {
        &self.connection_info().schema_name()
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.connector().database_info.connection_info()
    }

    fn conn(&self) -> &dyn Queryable {
        self.connector().database.as_ref()
    }

    fn database_info(&self) -> &DatabaseInfo {
        &self.connector().database_info
    }

    async fn describe(&self) -> SqlResult<SqlSchema> {
        self.connector().describe().await
    }

    fn sql_family(&self) -> SqlFamily {
        self.connection_info().sql_family()
    }
}

#[async_trait::async_trait]
impl Component for SqlMigrationConnector {
    fn connector(&self) -> &SqlMigrationConnector {
        self
    }
}

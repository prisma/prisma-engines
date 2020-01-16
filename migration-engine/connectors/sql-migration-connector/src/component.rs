use crate::{DatabaseInfo, SqlMigrationConnector, SqlResult};
use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};

#[async_trait::async_trait]
pub(crate) trait Component {
    fn connector(&self) -> &SqlMigrationConnector;

    fn schema_name(&self) -> &str {
        &self.connector().schema_name
    }

    fn connection_info(&self) -> &ConnectionInfo {
        self.connector().connection_info()
    }

    fn conn(&self) -> &dyn Queryable {
        self.connector().database.as_ref()
    }

    fn database_info(&self) -> &DatabaseInfo {
        &self.connector().database_info
    }

    async fn describe(&self) -> SqlResult<sql_schema_describer::SqlSchema> {
        Ok(self
            .connector()
            .database_describer
            .describe(&self.schema_name())
            .await?)
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

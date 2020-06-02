use crate::{DatabaseInfo, SqlMigrationConnector, SqlResult};
use quaint::prelude::{ConnectionInfo, Queryable, SqlFamily};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};

#[async_trait::async_trait]
pub(crate) trait Component {
    fn connector(&self) -> &SqlMigrationConnector;

    fn schema_name(&self) -> &str {
        &self.connection_info().schema_name()
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

    async fn describe(&self) -> SqlResult<SqlSchema> {
        let conn = self.connector().database.clone();
        let schema_name = self.schema_name();

        let schema = match self.connection_info().sql_family() {
            SqlFamily::Postgres => {
                sql_schema_describer::postgres::SqlSchemaDescriber::new(conn)
                    .describe(schema_name)
                    .await?
            }
            SqlFamily::Mysql => {
                sql_schema_describer::mysql::SqlSchemaDescriber::new(conn)
                    .describe(schema_name)
                    .await?
            }
            SqlFamily::Sqlite => {
                sql_schema_describer::sqlite::SqlSchemaDescriber::new(conn)
                    .describe(schema_name)
                    .await?
            }
        };

        Ok(schema)
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

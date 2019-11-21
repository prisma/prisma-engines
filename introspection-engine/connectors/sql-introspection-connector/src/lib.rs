pub mod calculate_datamodel; // only exported to be able to unit test it

mod error;
mod schema_describer_loading;

use datamodel::Datamodel;
use quaint::prelude::ConnectionInfo;
use introspection_connector::{ConnectorResult, ConnectorError, DatabaseMetadata, IntrospectionConnector};
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::future::Future;

pub use error::*;

pub type SqlIntrospectionResult<T> = core::result::Result<T, SqlIntrospectionError>;

pub struct SqlIntrospectionConnector {
    connection_info: ConnectionInfo,
    describer: Box<dyn SqlSchemaDescriberBackend>,
}

impl SqlIntrospectionConnector {
    pub fn new(url: &str) -> ConnectorResult<SqlIntrospectionConnector> {
        let (describer, connection_info) = schema_describer_loading::load_describer(&url).map_err(|quaint_error| {
            ConnectionInfo::from_url(url).map(|connection_info| SqlIntrospectionError::Quaint(quaint_error).into_connector_error(&connection_info)).unwrap_or_else(|err| ConnectorError::url_parse_error(err, url))
            })
        ?;
        Ok(SqlIntrospectionConnector { describer, connection_info })
    }

    async fn catch<O>(&self, fut: impl Future<Output=Result<O, SqlIntrospectionError>>) -> ConnectorResult<O> {
        fut.await.map_err(|sql_introspection_error| sql_introspection_error.into_connector_error(&self.connection_info))
    }

    async fn list_databases_internal(&self) -> SqlIntrospectionResult<Vec<String>> {
        Ok(self.describer.list_databases().await?)
    }

    async fn get_metadata_internal(&self, database: &str) -> SqlIntrospectionResult<DatabaseMetadata> {
        let sql_metadata = self.describer.get_metadata(&database).await?;
        let db_metadate = DatabaseMetadata {
            table_count: sql_metadata.table_count,
            size_in_bytes: sql_metadata.size_in_bytes,
        };
        Ok(db_metadate)
    }

    async fn describe(&self, database: &str) -> SqlIntrospectionResult<SqlSchema> {
        Ok(self.describer.describe(&database).await?)
    }
}

#[async_trait::async_trait]
impl IntrospectionConnector for SqlIntrospectionConnector {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>> {
        Ok(self.catch(self.list_databases_internal()).await?)
    }

    async fn get_metadata(&self, database: &str) -> ConnectorResult<DatabaseMetadata> {
        Ok(self.catch(self.get_metadata_internal(&database)).await?)
    }

    async fn introspect(&self, database: &str) -> ConnectorResult<Datamodel> {
        let sql_schema = self.catch(self.describe(database)).await?;
        let data_model = calculate_datamodel::calculate_model(&sql_schema).unwrap();
        Ok(data_model)
    }
}

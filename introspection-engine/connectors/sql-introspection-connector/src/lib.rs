pub mod calculate_datamodel; // only exported to be able to unit test it
mod error;
mod misc_helpers;
mod sanitize_datamodel_names; // only exported to be able to unit test it
mod schema_describer_loading;

use datamodel::Datamodel;
use introspection_connector::{ConnectorError, ConnectorResult, DatabaseMetadata, IntrospectionConnector};
use quaint::prelude::ConnectionInfo;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::future::Future;
use tracing_futures::Instrument;

pub use error::*;

pub type SqlIntrospectionResult<T> = core::result::Result<T, SqlIntrospectionError>;

pub struct SqlIntrospectionConnector {
    connection_info: ConnectionInfo,
    describer: Box<dyn SqlSchemaDescriberBackend>,
}

impl SqlIntrospectionConnector {
    pub async fn new(url: &str) -> ConnectorResult<SqlIntrospectionConnector> {
        let (describer, connection_info) = schema_describer_loading::load_describer(&url)
            .instrument(tracing::debug_span!("Loading describer"))
            .await
            .map_err(|quaint_error| {
                ConnectionInfo::from_url(url)
                    .map(|connection_info| {
                        SqlIntrospectionError::Quaint(quaint_error).into_connector_error(&connection_info)
                    })
                    .unwrap_or_else(|err| ConnectorError::url_parse_error(err, url))
            })?;

        tracing::debug!("SqlIntrospectionConnector initialized.");

        Ok(SqlIntrospectionConnector {
            describer,
            connection_info,
        })
    }

    async fn catch<O>(&self, fut: impl Future<Output = Result<O, SqlIntrospectionError>>) -> ConnectorResult<O> {
        fut.await
            .map_err(|sql_introspection_error| sql_introspection_error.into_connector_error(&self.connection_info))
    }

    async fn list_databases_internal(&self) -> SqlIntrospectionResult<Vec<String>> {
        Ok(self.describer.list_databases().await?)
    }

    async fn get_metadata_internal(&self) -> SqlIntrospectionResult<DatabaseMetadata> {
        let sql_metadata = self.describer.get_metadata(self.connection_info.schema_name()).await?;
        let db_metadate = DatabaseMetadata {
            table_count: sql_metadata.table_count,
            size_in_bytes: sql_metadata.size_in_bytes,
        };
        Ok(db_metadate)
    }

    async fn describe(&self) -> SqlIntrospectionResult<SqlSchema> {
        Ok(self.describer.describe(self.connection_info.schema_name()).await?)
    }
}

#[async_trait::async_trait]
impl IntrospectionConnector for SqlIntrospectionConnector {
    async fn list_databases(&self) -> ConnectorResult<Vec<String>> {
        Ok(self.catch(self.list_databases_internal()).await?)
    }

    async fn get_metadata(&self) -> ConnectorResult<DatabaseMetadata> {
        Ok(self.catch(self.get_metadata_internal()).await?)
    }

    async fn introspect(&self) -> ConnectorResult<Datamodel> {
        let sql_schema = self.catch(self.describe()).await?;
        let data_model = calculate_datamodel::calculate_model(&sql_schema).unwrap();
        Ok(data_model)
    }
}

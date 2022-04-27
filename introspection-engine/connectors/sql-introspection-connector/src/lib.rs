#![allow(clippy::vec_init_then_push)]
#![allow(clippy::ptr_arg)] // remove after https://github.com/rust-lang/rust-clippy/issues/8482 is fixed and shipped

pub mod calculate_datamodel; // only exported to be able to unit test it

mod calculate_datamodel_tests;
mod commenting_out_guardrails;
mod error;
mod introspection;
mod introspection_helpers;
mod prisma_1_defaults;
mod re_introspection;
mod sanitize_datamodel_names;
mod schema_describer_loading;
mod version_checker;
mod warnings;

pub use error::*;

use datamodel::{common::preview_features::PreviewFeature, dml::Datamodel};
use enumflags2::BitFlags;
use introspection_connector::{
    ConnectorError, ConnectorResult, DatabaseMetadata, ErrorKind, IntrospectionConnector, IntrospectionContext,
    IntrospectionResult,
};
use quaint::prelude::SqlFamily;
use quaint::{prelude::ConnectionInfo, single::Quaint};
use schema_describer_loading::load_describer;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::future::Future;
use user_facing_errors::common::InvalidConnectionString;
use user_facing_errors::KnownError;

pub type SqlIntrospectionResult<T> = core::result::Result<T, SqlError>;

#[derive(Debug)]
pub struct SqlIntrospectionConnector {
    connection: Quaint,
    preview_features: BitFlags<PreviewFeature>,
}

impl SqlIntrospectionConnector {
    pub async fn new(
        connection_string: &str,
        preview_features: BitFlags<PreviewFeature>,
    ) -> ConnectorResult<SqlIntrospectionConnector> {
        let connection = Quaint::new(connection_string).await.map_err(|error| {
            ConnectionInfo::from_url(connection_string)
                .map(|connection_info| SqlError::from(error).into_connector_error(&connection_info))
                .unwrap_or_else(|err| {
                    let details = user_facing_errors::quaint::invalid_connection_string_description(&err.to_string());
                    let known = KnownError::new(InvalidConnectionString { details });

                    ConnectorError {
                        user_facing_error: Some(known),
                        kind: ErrorKind::InvalidDatabaseUrl(format!("{} in database URL", err)),
                    }
                })
        })?;

        tracing::debug!("SqlIntrospectionConnector initialized.");

        Ok(SqlIntrospectionConnector {
            connection,
            preview_features,
        })
    }

    async fn catch<O>(&self, fut: impl Future<Output = Result<O, SqlError>>) -> ConnectorResult<O> {
        fut.await.map_err(|sql_introspection_error| {
            sql_introspection_error.into_connector_error(self.connection.connection_info())
        })
    }

    async fn describer(
        &self,
        provider: Option<&str>,
    ) -> SqlIntrospectionResult<Box<dyn SqlSchemaDescriberBackend + '_>> {
        load_describer(
            &self.connection,
            self.connection.connection_info(),
            provider,
            self.preview_features,
        )
        .await
    }

    async fn list_databases_internal(&self) -> SqlIntrospectionResult<Vec<String>> {
        Ok(self.describer(None).await?.list_databases().await?)
    }

    async fn get_metadata_internal(&self) -> SqlIntrospectionResult<DatabaseMetadata> {
        let sql_metadata = self
            .describer(None)
            .await?
            .get_metadata(self.connection.connection_info().schema_name())
            .await?;

        let db_metadate = DatabaseMetadata {
            table_count: sql_metadata.table_count,
            size_in_bytes: sql_metadata.size_in_bytes,
        };

        Ok(db_metadate)
    }

    /// Exported for tests
    pub fn quaint(&self) -> &Quaint {
        &self.connection
    }

    /// Exported for tests
    pub async fn describe(&self, provider: Option<&str>) -> SqlIntrospectionResult<SqlSchema> {
        Ok(self
            .describer(provider)
            .await?
            .describe(self.connection.connection_info().schema_name())
            .await?)
    }

    async fn version(&self) -> SqlIntrospectionResult<String> {
        Ok(self
            .describer(None)
            .await?
            .version(self.connection.connection_info().schema_name())
            .await?
            .unwrap_or_else(|| "Database version information not available.".into()))
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

    async fn get_database_description(&self) -> ConnectorResult<String> {
        let sql_schema = self.catch(self.describe(None)).await?;
        tracing::debug!("SQL Schema Describer is done: {:?}", sql_schema);
        let description = serde_json::to_string_pretty(&sql_schema).unwrap();
        Ok(description)
    }

    async fn get_database_version(&self) -> ConnectorResult<String> {
        let sql_schema = self.catch(self.version()).await?;
        tracing::debug!("Fetched db version for: {:?}", sql_schema);
        let description = serde_json::to_string(&sql_schema).unwrap();
        Ok(description)
    }

    async fn introspect(
        &self,
        previous_data_model: &Datamodel,
        ctx: IntrospectionContext,
    ) -> ConnectorResult<IntrospectionResult> {
        let sql_schema = self.catch(self.describe(Some(&ctx.source.active_provider))).await?;
        tracing::debug!("SQL Schema Describer is done: {:?}", sql_schema);

        let introspection_result = calculate_datamodel::calculate_datamodel(&sql_schema, previous_data_model, ctx)
            .map_err(|sql_introspection_error| {
                sql_introspection_error.into_connector_error(self.connection.connection_info())
            })?;

        tracing::debug!("Calculating datamodel is done: {:?}", introspection_result.data_model);

        Ok(introspection_result)
    }
}

trait Dedup<T: PartialEq + Clone> {
    fn clear_duplicates(&mut self);
}

impl<T: PartialEq + Clone> Dedup<T> for Vec<T> {
    fn clear_duplicates(&mut self) {
        let mut already_seen = vec![];
        self.retain(|item| match already_seen.contains(item) {
            true => false,
            _ => {
                already_seen.push(item.clone());
                true
            }
        })
    }
}

trait SqlFamilyTrait {
    fn sql_family(&self) -> SqlFamily;
}

impl SqlFamilyTrait for IntrospectionContext {
    fn sql_family(&self) -> SqlFamily {
        match self.source.active_provider.as_str() {
            "postgresql" => SqlFamily::Postgres,
            "cockroachdb" => SqlFamily::Postgres,
            "sqlite" => SqlFamily::Sqlite,
            "sqlserver" => SqlFamily::Mssql,
            "mysql" => SqlFamily::Mysql,
            name => unreachable!("The name `{}` for the datamodel connector is not known", name),
        }
    }
}

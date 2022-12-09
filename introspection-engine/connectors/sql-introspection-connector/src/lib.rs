//! Handles the introspection from SQL database schema definition to
//! the Prisma Schema Language (PSL).
//!
//! The introspection consists the following parts:
//!
//! - Describe the database schema using the `sql-schema-describer`.
//! - Combine the describe result with a parsed and validated PSL
//!   definition, happens in [`introspection_map::IntrospectionMap`]
//! - Using the `Pair` apis, provide information about the
//!   data model for rendering, see [`pair::Pair`] and its submodules.
//! - By using the `Pair` apis, create a rendering structure together
//!   with possible warnings utilizing the `datamodel-renderer`
//!   crate in the [`rendering`] module.
//! - Check the Prisma version to warn and guide people upgrading
//!   from older versions of Prisma.
//! - Convert the rendering structure into a string. Reformat the
//!   string using the `psl::reformat` module.
//!
//! A good place to start digging into the mechanics of this crate is
//! to start reading from the
//! [`SqlIntrospectionConnector::introspect`] method.

#![allow(clippy::vec_init_then_push)]
#![allow(clippy::ptr_arg)] // remove after https://github.com/rust-lang/rust-clippy/issues/8482 is fixed and shipped

pub mod datamodel_calculator; // only exported to be able to unit test it
mod pair;

mod error;
mod introspection_helpers;
mod introspection_map;
mod rendering;
mod sanitize_datamodel_names;
mod schema_describer_loading;
mod version_checker;
mod warnings;

pub use error::*;

use self::sanitize_datamodel_names::*;
use enumflags2::BitFlags;
use introspection_connector::{
    ConnectorError, ConnectorResult, DatabaseMetadata, ErrorKind, IntrospectionConnector, IntrospectionContext,
    IntrospectionResult,
};
use psl::PreviewFeature;
use quaint::{
    prelude::{ConnectionInfo, SqlFamily},
    single::Quaint,
};
use schema_describer_loading::load_describer;
use sql_schema_describer::{SqlSchema, SqlSchemaDescriberBackend};
use std::future::Future;
use user_facing_errors::common::InvalidConnectionString;
use user_facing_errors::KnownError;

pub type SqlIntrospectionResult<T> = core::result::Result<T, SqlError>;

#[derive(Debug)]
pub struct SqlIntrospectionConnector {
    connection: Quaint,
    _preview_features: BitFlags<PreviewFeature>,
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
            _preview_features: preview_features,
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
        load_describer(&self.connection, self.connection.connection_info(), provider).await
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
    pub async fn describe(&self, provider: Option<&str>, namespaces: &[&str]) -> SqlIntrospectionResult<SqlSchema> {
        Ok(self.describer(provider).await?.describe(namespaces).await?)
    }

    async fn version(&self) -> SqlIntrospectionResult<String> {
        Ok(self
            .describer(None)
            .await?
            .version()
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
        let sql_schema = self
            .catch(self.describe(None, &[self.connection.connection_info().schema_name()]))
            .await?;
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

    async fn introspect(&self, ctx: &IntrospectionContext) -> ConnectorResult<IntrospectionResult> {
        let namespaces = &mut ctx
            .datasource()
            .namespaces
            .iter()
            .map(|(ns, _)| ns.as_ref())
            .collect::<Vec<&str>>();

        if namespaces.is_empty() {
            namespaces.push(self.connection.connection_info().schema_name())
        }

        let sql_schema = self
            .catch(self.describe(Some(ctx.datasource().active_provider), namespaces))
            .await?;

        datamodel_calculator::calculate(&sql_schema, ctx).map_err(|sql_introspection_error| {
            sql_introspection_error.into_connector_error(self.connection.connection_info())
        })
    }
}

trait SqlFamilyTrait {
    fn sql_family(&self) -> SqlFamily;
}

impl SqlFamilyTrait for IntrospectionContext {
    fn sql_family(&self) -> SqlFamily {
        match self.datasource().active_provider {
            "postgresql" => SqlFamily::Postgres,
            "cockroachdb" => SqlFamily::Postgres,
            "sqlite" => SqlFamily::Sqlite,
            "sqlserver" => SqlFamily::Mssql,
            "mysql" => SqlFamily::Mysql,
            name => unreachable!("The name `{}` for the datamodel connector is not known", name),
        }
    }
}

impl SqlFamilyTrait for datamodel_calculator::InputContext<'_> {
    fn sql_family(&self) -> SqlFamily {
        self.sql_family
    }
}

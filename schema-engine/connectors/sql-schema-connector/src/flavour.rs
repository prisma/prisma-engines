//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

#[cfg(feature = "mssql")]
mod mssql;

#[cfg(feature = "mysql")]
mod mysql;

#[cfg(any(feature = "postgresql", feature = "cockroachdb"))]
mod postgres;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "mssql")]
pub(crate) use mssql::{MssqlConnector, MssqlDialect};

#[cfg(feature = "mysql")]
pub(crate) use mysql::{MysqlConnector, MysqlDialect};

#[cfg(any(feature = "postgresql", feature = "cockroachdb"))]
pub(crate) use postgres::{PostgresConnector, PostgresDialect};

#[cfg(feature = "sqlite")]
pub(crate) use sqlite::{SqliteConnector, SqliteDialect};

use crate::{
    sql_destructive_change_checker::DestructiveChangeCheckerFlavour, sql_renderer::SqlRenderer,
    sql_schema_calculator::SqlSchemaCalculatorFlavour, sql_schema_differ::SqlSchemaDifferFlavour,
};
use psl::{PreviewFeatures, ValidatedSchema};
use quaint::prelude::{NativeConnectionInfo, Table};
use schema_connector::{
    BoxFuture, ConnectorError, ConnectorResult, IntrospectionContext, MigrationRecord, Namespaces,
    PersistenceNotInitializedError, SchemaFilter, migrations_directory::Migrations,
};
use sql_schema_describer::SqlSchema;
use std::fmt::Debug;

/// P is the params, C is a connection.
pub(crate) enum State<P, C> {
    Initial,
    WithParams(P),
    Connected(P, C),
}

impl<P, C> Debug for State<P, C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            State::Initial => f.write_str("State::Initial"),
            State::WithParams(_) => f.write_str("State::Params(<CONFIDENTIAL>)"),
            State::Connected(_, _) => f.write_str("State::Connected(<CONFIDENTIAL>)"),
        }
    }
}

impl<P, C> State<P, C>
where
    P: 'static,
    C: 'static,
{
    fn params(&self) -> Option<&P> {
        match self {
            State::Initial => None,
            State::WithParams(p) | State::Connected(p, _) => Some(p),
        }
    }

    /// Unwrap the state's params. We do not return an error because we want to trigger a panic if
    /// that happens, because it means an internal logic error.
    ///
    /// This is useful when you want the params, but you do not care if a connection has been
    /// started or not.
    #[track_caller]
    fn get_unwrapped_params(&self) -> &P {
        match self {
            State::Initial => panic!("Internal logic error: get_unwrapped_params() on State::Initial"),
            State::WithParams(p) => p,
            State::Connected(p, _) => p,
        }
    }

    /// Convenience wrapper to transition from WithParams to Connected.
    async fn try_connect(
        &mut self,
        f: impl for<'b> FnOnce(&'b P) -> BoxFuture<'b, ConnectorResult<C>>,
    ) -> ConnectorResult<()> {
        match std::mem::replace(self, State::Initial) {
            State::Initial => panic!("Attempted to connect from State::Initial"),
            State::Connected(_, _) => panic!("Attempted to connect from State::Connected"),
            State::WithParams(p) => match f(&p).await {
                Ok(c) => {
                    *self = State::Connected(p, c);
                    Ok(())
                }
                Err(err) => {
                    *self = State::WithParams(p);
                    Err(err)
                }
            },
        }
    }
}

pub(crate) trait SqlDialect: Send + Sync + 'static {
    fn renderer(&self) -> Box<dyn SqlRenderer>;
    fn schema_differ(&self) -> Box<dyn SqlSchemaDifferFlavour>;
    fn schema_calculator(&self) -> Box<dyn SqlSchemaCalculatorFlavour>;
    fn destructive_change_checker(&self) -> Box<dyn DestructiveChangeCheckerFlavour>;

    /// Check a schema for preview features not implemented in migrate/introspection.
    fn check_schema_features(&self, _schema: &psl::ValidatedSchema) -> ConnectorResult<()> {
        Ok(())
    }

    /// The datamodel connector corresponding to the dialect.
    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector;

    /// Return an empty database schema.
    fn empty_database_schema(&self) -> SqlSchema {
        SqlSchema::default()
    }

    /// The default namespace for the dialect if it supports multiple namespaces.
    fn default_namespace(&self) -> Option<&str> {
        None
    }

    /// Optionally scan a migration script that could have been altered by users and emit warnings.
    fn scan_migration_script(&self, _script: &str) {}

    /// Table to store applied migrations.
    fn migrations_table(&self) -> Table<'static> {
        crate::MIGRATIONS_TABLE_NAME.into()
    }

    #[cfg(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    ))]
    fn connect_to_shadow_db(
        &self,
        url: String,
        preview_features: PreviewFeatures,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>>;

    #[cfg(not(any(
        feature = "mssql-native",
        feature = "mysql-native",
        feature = "postgresql-native",
        feature = "sqlite-native"
    )))]
    fn connect_to_shadow_db(
        &self,
        factory: std::sync::Arc<dyn quaint::connector::ExternalConnectorFactory>,
    ) -> BoxFuture<'_, ConnectorResult<Box<dyn SqlConnector>>>;
}

pub(crate) trait SqlConnector: Send + Sync + Debug {
    fn dialect(&self) -> Box<dyn SqlDialect>;

    fn shadow_db_url(&self) -> Option<&str>;

    fn acquire_lock(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    fn apply_migration_script<'a>(
        &'a mut self,
        migration_name: &'a str,
        script: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<()>>;

    fn check_database_version_compatibility(
        &self,
        _datamodel: &ValidatedSchema,
    ) -> Option<user_facing_errors::common::DatabaseVersionIncompatibility> {
        None
    }

    /// See MigrationConnector::connector_type()
    fn connector_type(&self) -> &'static str;

    /// Create a database for the given URL on the server, if applicable.
    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>>;

    /// Initialize the `_prisma_migrations` table.
    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    fn describe_schema(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>>;

    /// Drop the database.
    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Drop the migrations table
    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// List all visible tables in the given namespaces,
    /// including the search path.
    fn table_names(
        &mut self,
        namespaces: Option<Namespaces>,
        filters: SchemaFilter,
    ) -> BoxFuture<'_, ConnectorResult<Vec<String>>>;

    /// Check a connection to make sure it is usable by the schema engine.
    /// This can include some set up on the database, like ensuring that the
    /// schema we connect to exists.
    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Same as [describe_schema], but only called for introspection.
    fn introspect<'a>(
        &'a mut self,
        namespaces: Option<Namespaces>,
        _ctx: &'a IntrospectionContext,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>> {
        self.describe_schema(namespaces)
    }

    fn describe_query<'a>(
        &'a mut self,
        sql: &'a str,
    ) -> BoxFuture<'a, ConnectorResult<quaint::connector::DescribedQuery>>;

    fn load_migrations_table(
        &mut self,
    ) -> BoxFuture<'_, ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>>> {
        use quaint::prelude::*;
        Box::pin(async move {
            let select = Select::from_table(self.dialect().migrations_table())
                .column("id")
                .column("checksum")
                .column("finished_at")
                .column("migration_name")
                .column("logs")
                .column("rolled_back_at")
                .column("started_at")
                .column("applied_steps_count")
                .order_by("started_at".ascend());

            let rows = match self.query(select.into()).await {
                Ok(result) => result,
                Err(err)
                    if err.is_user_facing_error::<user_facing_errors::query_engine::TableDoesNotExist>()
                        || err.is_user_facing_error::<user_facing_errors::common::InvalidModel>() =>
                {
                    return Ok(Err(PersistenceNotInitializedError));
                }
                Err(_) => {
                    // TODO: this is a workaround, as currently the errors thrown by Driver Adapters do not
                    // match the known user-facing errors we expect.
                    // We should fix this in the future.
                    //
                    // This used to actually yield:
                    // ```
                    // err @ Err(_) => err?
                    // ```
                    return Ok(Err(PersistenceNotInitializedError));
                }
            };

            let rows = rows
                .into_iter()
                .map(|row| -> ConnectorResult<_> {
                    Ok(MigrationRecord {
                        id: row.get("id").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg("Failed to extract `id` from `_prisma_migrations` row.".into())
                        })?,
                        checksum: row.get("checksum").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `checksum` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        finished_at: row.get("finished_at").and_then(|v| v.as_datetime()),
                        migration_name: row.get("migration_name").and_then(|v| v.to_string()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `migration_name` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        logs: None,
                        rolled_back_at: row.get("rolled_back_at").and_then(|v| v.as_datetime()),
                        started_at: row.get("started_at").and_then(|v| v.as_datetime()).ok_or_else(|| {
                            ConnectorError::from_msg(
                                "Failed to extract `started_at` from `_prisma_migrations` row.".into(),
                            )
                        })?,
                        applied_steps_count: row.get("applied_steps_count").and_then(|v| v.as_integer()).ok_or_else(
                            || {
                                ConnectorError::from_msg(
                                    "Failed to extract `applied_steps_count` from `_prisma_migrations` row.".into(),
                                )
                            },
                        )? as u32,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;

            tracing::debug!("Found {} migrations in the migrations table.", rows.len());

            Ok(Ok(rows))
        })
    }

    fn query<'a>(
        &'a mut self,
        query: quaint::ast::Query<'a>,
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>>;

    fn query_raw<'a>(
        &'a mut self,
        sql: &'a str,
        params: &'a [quaint::prelude::Value<'a>],
    ) -> BoxFuture<'a, ConnectorResult<quaint::prelude::ResultSet>>;

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Drop the database and recreate it empty.
    fn reset(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Apply the given migration history to a shadow database, and return
    /// the final introspected SQL schema. The third parameter specifies whether an external
    /// shadow database is being used - if not, we need to create a temporary one.
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a Migrations,
        namespaces: Option<Namespaces>,
        filter: &'a SchemaFilter,
        external_shadow_db: UsingExternalShadowDb,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>>;

    /// Sets the preview features. This is currently useful for MultiSchema, as we want to
    /// grab the namespaces we're expected to diff/work on, which are generally set in
    /// the schema.
    /// WARNING: This may silently not do anything if the connector is in the initial state.
    /// If this is ever a problem, considering returning an indicator of success.
    fn set_preview_features(&mut self, preview_features: PreviewFeatures);

    fn preview_features(&self) -> PreviewFeatures;

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>>;

    fn search_path(&self) -> &str;

    /// The default namespaces for the connector if it supports multiple namespaces.
    /// Should be derived from the connectors runtime configuration but can fallback to the dialect's default.
    fn default_namespace(&self) -> Option<&str>;

    fn dispose(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;
}

// Utility function shared by multiple dialects to compare shadow database and main connection.
fn validate_connection_infos_do_not_match(previous: &str, next: &str) -> ConnectorResult<()> {
    if previous == next {
        Err(ConnectorError::from_msg("The shadow database you configured appears to be the same as the main database. Please specify another shadow database.".into()))
    } else {
        Ok(())
    }
}

/// Remove all usage of non-enabled preview feature elements from the SqlSchema.
fn normalize_sql_schema(sql_schema: &mut SqlSchema, _preview_features: BitFlags<PreviewFeature>) {
    sql_schema.clear_namespaces();
}

pub(crate) fn quaint_error_to_connector_error(
    error: quaint::error::Error,
    connection_info: Option<&NativeConnectionInfo>,
) -> ConnectorError {
    match user_facing_errors::quaint::render_quaint_error(error.kind(), connection_info) {
        Some(user_facing_error) => user_facing_error.into(),
        None => {
            let msg = error
                .original_message()
                .map(String::from)
                .unwrap_or_else(|| error.to_string());
            ConnectorError::from_msg(msg)
        }
    }
}

/// A flag that indicates whether the connector is using an external shadow database.
#[derive(Debug)]
pub enum UsingExternalShadowDb {
    /// We're using an external shadow database (such as a custom-provided connection string
    /// or a JavaScript adapter). This indicates that it can be safely written to for schema
    /// calculation purposes.
    Yes,
    /// We're not using an external shadow database. When this is the case, the connector must
    /// create a new temporary database for schema calculation purposes.
    No,
}

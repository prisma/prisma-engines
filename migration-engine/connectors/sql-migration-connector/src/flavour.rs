//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

pub(crate) use mssql::MssqlFlavour;
pub(crate) use mysql::MysqlFlavour;
pub(crate) use postgres::PostgresFlavour;
pub(crate) use sqlite::SqliteFlavour;

use crate::{
    sql_destructive_change_checker::DestructiveChangeCheckerFlavour, sql_renderer::SqlRenderer,
    sql_schema_calculator::SqlSchemaCalculatorFlavour, sql_schema_differ::SqlSchemaDifferFlavour,
};
use enumflags2::BitFlags;
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
    MigrationRecord, Namespaces, PersistenceNotInitializedError,
};
use psl::{PreviewFeature, ValidatedSchema};
use quaint::prelude::{ConnectionInfo, Table};
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

    #[track_caller]
    fn set_params(&mut self, params: P) {
        match self {
            State::WithParams(_) | State::Connected(_, _) => panic!("state error"),
            State::Initial => *self = State::WithParams(params),
        }
    }

    /// Convenience wrapper to transition from WithParams to Connected.
    #[track_caller]
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

pub(crate) trait SqlFlavour:
    DestructiveChangeCheckerFlavour
    + SqlRenderer
    + SqlSchemaDifferFlavour
    + SqlSchemaCalculatorFlavour
    + Send
    + Sync
    + Debug
{
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

    /// Check a schema for preview features not implemented in migrate/introspection.
    fn check_schema_features(&self, _schema: &psl::ValidatedSchema) -> ConnectorResult<()> {
        Ok(())
    }

    /// The connection string received in set_params().
    fn connection_string(&self) -> Option<&str>;

    /// See MigrationConnector::connector_type()
    fn connector_type(&self) -> &'static str;

    /// Create a database for the given URL on the server, if applicable.
    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>>;

    /// Initialize the `_prisma_migrations` table.
    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// The datamodel connector corresponding to the flavour
    fn datamodel_connector(&self) -> &'static dyn psl::datamodel_connector::Connector;

    fn describe_schema(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<SqlSchema>>;

    /// Drop the database.
    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Drop the migrations table
    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Return an empty database schema. This happens in the flavour, because we need
    /// SqlSchema::connector_data to be set.
    fn empty_database_schema(&self) -> SqlSchema {
        SqlSchema::default()
    }

    /// Check a connection to make sure it is usable by the migration engine.
    /// This can include some set up on the database, like ensuring that the
    /// schema we connect to exists.
    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    fn load_migrations_table(
        &mut self,
    ) -> BoxFuture<'_, ConnectorResult<Result<Vec<MigrationRecord>, PersistenceNotInitializedError>>> {
        use quaint::prelude::*;
        Box::pin(async move {
            let select = Select::from_table(self.migrations_table())
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
                    return Ok(Err(PersistenceNotInitializedError))
                }
                err @ Err(_) => err?,
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
    ) -> BoxFuture<'_, ConnectorResult<quaint::prelude::ResultSet>>;

    fn raw_cmd<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Drop the database and recreate it empty.
    fn reset(&mut self, namespaces: Option<Namespaces>) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Optionally scan a migration script that could have been altered by users and emit warnings.
    fn scan_migration_script(&self, _script: &str) {}

    /// Apply the given migration history to a shadow database, and return
    /// the final introspected SQL schema. The third parameter is an optional shadow database url
    /// in case there is one at this point of the command, but not earlier in set_params().
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        shadow_database_url: Option<String>,
        namespaces: Option<Namespaces>,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>>;

    /// Receive and validate connector params.
    fn set_params(&mut self, connector_params: ConnectorParams) -> ConnectorResult<()>;

    /// Sets the preview features. This is currently useful for MultiSchema, as we want to
    /// grab the namespaces we're expected to diff/work on, which are generally set in
    /// the schema.
    /// WARNING: This may silently not do anything if the connector is in the initial state.
    /// If this is ever a problem, considering returning an indicator of success.
    fn set_preview_features(&mut self, preview_features: BitFlags<psl::PreviewFeature>);

    /// Table to store applied migrations.
    fn migrations_table(&self) -> Table<'static> {
        crate::MIGRATIONS_TABLE_NAME.into()
    }

    fn version(&mut self) -> BoxFuture<'_, ConnectorResult<Option<String>>>;
}

// Utility function shared by multiple flavours to compare shadow database and main connection.
fn validate_connection_infos_do_not_match(previous: &str, next: &str) -> ConnectorResult<()> {
    if previous == next {
        Err(ConnectorError::from_msg("The shadow database you configured appears to be the same as the main database. Please specify another shadow database.".into()))
    } else {
        Ok(())
    }
}

/// Remove all usage of non-enabled preview feature elements from the SqlSchema.
fn normalize_sql_schema(sql_schema: &mut SqlSchema, preview_features: BitFlags<PreviewFeature>) {
    // Remove this when the feature is GA
    if !preview_features.contains(PreviewFeature::FullTextIndex) {
        sql_schema.make_fulltext_indexes_normal();
    }

    if !preview_features.contains(PreviewFeature::MultiSchema) {
        sql_schema.clear_namespaces();
    }
}

fn quaint_error_to_connector_error(error: quaint::error::Error, connection_info: &ConnectionInfo) -> ConnectorError {
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

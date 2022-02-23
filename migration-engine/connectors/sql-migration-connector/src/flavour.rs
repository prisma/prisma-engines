//! SQL flavours implement behaviour specific to a given SQL implementation (PostgreSQL, SQLite...),
//! in order to avoid cluttering the connector with conditionals. This is a private implementation
//! detail of the SQL connector.

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use enumflags2::BitFlags;
pub(crate) use mssql::MssqlFlavour;
pub(crate) use mysql::MysqlFlavour;
pub(crate) use postgres::PostgresFlavour;
pub(crate) use sqlite::SqliteFlavour;

use crate::{
    connection_wrapper::Connection, sql_destructive_change_checker::DestructiveChangeCheckerFlavour,
    sql_renderer::SqlRenderer, sql_schema_calculator::SqlSchemaCalculatorFlavour,
    sql_schema_differ::SqlSchemaDifferFlavour,
};
use datamodel::{common::preview_features::PreviewFeature, ValidatedSchema};
use migration_connector::{
    migrations_directory::MigrationDirectory, BoxFuture, ConnectorError, ConnectorParams, ConnectorResult,
};
use quaint::prelude::Table;
use sql_schema_describer::SqlSchema;
use std::fmt::Debug;
use user_facing_errors::migration_engine::ApplyMigrationError;

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
    DestructiveChangeCheckerFlavour + SqlRenderer + SqlSchemaDifferFlavour + SqlSchemaCalculatorFlavour + Debug
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

    /// The connection string received in set_params().
    fn connection_string(&self) -> Option<&str>;

    /// See MigrationConnector::connector_type()
    fn connector_type(&self) -> &'static str;

    /// Create a database for the given URL on the server, if applicable.
    fn create_database(&mut self) -> BoxFuture<'_, ConnectorResult<String>>;

    /// Initialize the `_prisma_migrations` table.
    fn create_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// The datamodel connector corresponding to the flavour
    fn datamodel_connector(&self) -> &'static dyn datamodel::datamodel_connector::Connector;

    fn describe_schema(&mut self) -> BoxFuture<'_, ConnectorResult<SqlSchema>>;

    /// Drop the database.
    fn drop_database(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Drop the migrations table
    fn drop_migrations_table(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Check a connection to make sure it is usable by the migration engine.
    /// This can include some set up on the database, like ensuring that the
    /// schema we connect to exists.
    fn ensure_connection_validity(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

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
    fn reset(&mut self) -> BoxFuture<'_, ConnectorResult<()>>;

    /// Optionally scan a migration script that could have been altered by users and emit warnings.
    fn scan_migration_script(&self, _script: &str) {}

    /// Apply the given migration history to a shadow database, and return
    /// the final introspected SQL schema. The third parameter is an optional shadow database url
    /// in case there is one at this point of the command, but not earlier in set_params().
    fn sql_schema_from_migration_history<'a>(
        &'a mut self,
        migrations: &'a [MigrationDirectory],
        shadow_database_url: Option<String>,
    ) -> BoxFuture<'a, ConnectorResult<SqlSchema>>;

    /// Runs a single SQL script.
    fn run_query_script<'a>(&'a mut self, sql: &'a str) -> BoxFuture<'a, ConnectorResult<()>>;

    /// Receive and validate connector params.
    fn set_params(&mut self, connector_params: ConnectorParams) -> ConnectorResult<()>;

    /// Table to store applied migrations, the name part.
    fn migrations_table_name(&self) -> &'static str {
        "_prisma_migrations"
    }

    /// Table to store applied migrations.
    fn migrations_table(&self) -> Table<'static> {
        self.migrations_table_name().into()
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

async fn generic_apply_migration_script(migration_name: &str, script: &str, conn: &Connection) -> ConnectorResult<()> {
    conn.raw_cmd(script).await.map_err(|sql_error| {
        ConnectorError::user_facing(ApplyMigrationError {
            migration_name: migration_name.to_owned(),
            database_error_code: String::from(sql_error.error_code().unwrap_or("none")),
            database_error: ConnectorError::from(sql_error).to_string(),
        })
    })
}

/// Remove all usage of non-enabled preview feature elements from the SqlSchema.
fn normalize_sql_schema(sql_schema: &mut SqlSchema, preview_features: BitFlags<PreviewFeature>) {
    use sql_schema_describer::IndexType;

    fn filter_fulltext_capabilities(schema: &mut SqlSchema) {
        let indices = schema
            .iter_tables_mut()
            .flat_map(|(_, t)| t.indices.iter_mut().filter(|i| i.is_fulltext()));

        for index in indices {
            index.tpe = IndexType::Normal;
        }
    }

    fn filter_extended_index_capabilities(schema: &mut SqlSchema) {
        for (_, table) in schema.iter_tables_mut() {
            if let Some(ref mut pk) = &mut table.primary_key {
                for col in pk.columns.iter_mut() {
                    col.length = None;
                    col.sort_order = None;
                }
            }

            let mut kept_indexes = Vec::new();

            while let Some(mut index) = table.indices.pop() {
                let mut remove_index = false;

                for col in index.columns.iter_mut() {
                    if col.length.is_some() {
                        remove_index = true;
                    }

                    col.sort_order = None;
                }

                index.algorithm = None;

                if !remove_index {
                    kept_indexes.push(index);
                }
            }

            kept_indexes.reverse();
            table.indices = kept_indexes;
        }
    }

    // Remove this when the feature is GA
    if !preview_features.contains(PreviewFeature::ExtendedIndexes) {
        filter_extended_index_capabilities(sql_schema);
    }

    // Remove this when the feature is GA
    if !preview_features.contains(PreviewFeature::FullTextIndex) {
        filter_fulltext_capabilities(sql_schema);
    }
}

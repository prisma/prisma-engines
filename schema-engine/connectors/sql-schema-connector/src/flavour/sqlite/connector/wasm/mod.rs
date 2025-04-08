//! All the quaint-wrangling for the sqlite connector should happen here.

use crate::flavour::sqlite::SqlSchemaDescriber;
use crate::BitFlags;
use psl::PreviewFeature;
use quaint::connector::{AdapterName, ExternalConnector};
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::{DescriberErrorKind, SqlSchema};
use std::sync::Arc;

pub struct State {
    connection: Connection,
    preview_features: BitFlags<PreviewFeature>,
}

impl State {
    pub fn new(adapter: Arc<dyn ExternalConnector>, preview_features: BitFlags<PreviewFeature>) -> Self {
        Self {
            preview_features,
            connection: Connection { adapter },
        }
    }
}

pub struct Params;

pub struct Connection {
    adapter: Arc<dyn ExternalConnector>,
}

impl Connection {
    pub fn as_connector(&self) -> &Arc<dyn ExternalConnector> {
        &self.adapter
    }

    pub fn adapter_name(&self) -> Option<AdapterName> {
        Some(self.adapter.adapter_name())
    }

    pub async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.adapter.raw_cmd(sql).await.map_err(convert_error)
    }

    pub async fn query(&self, query: quaint::ast::Query<'_>) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Sqlite::build(query).unwrap();
        self.query_raw(&sql, &params).await
    }

    pub async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        self.adapter.query_raw(sql, params).await.map_err(convert_error)
    }

    pub async fn version(&self) -> ConnectorResult<Option<String>> {
        match self.adapter.adapter_name() {
            // Cloudflare D1 doesn't allow querying the version.
            // We thus return a hardcoded string to avoid the error
            // `not authorized to use function: sqlite_version at offset`.
            AdapterName::D1(..) => Ok(Some("cf-d1".to_owned())),
            _ => self.adapter.version().await.map_err(convert_error),
        }
    }

    pub async fn describe_query(
        &self,
        sql: &str,
        _params: &Params,
    ) -> ConnectorResult<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        self.adapter.describe_query(sql).await.map_err(convert_error)
    }

    pub async fn apply_migration_script(&self, _migration_name: &str, _script: &str) -> ConnectorResult<()> {
        self.adapter.execute_script(_script).await.map_err(convert_error)
    }

    pub async fn reset(&self, _params: &Params) -> ConnectorResult<()> {
        let mut schema = SqlSchema::default();
        let container_ids = SqlSchemaDescriber::new(self.as_connector())
            .get_table_names(&mut schema)
            .await
            .map_err(|err| match err.into_kind() {
                DescriberErrorKind::QuaintError(err) => {
                    ConnectorError::from_source(err, "Error describing the database.")
                }
                DescriberErrorKind::CrossSchemaReference { .. } => {
                    unreachable!("No schemas on SQLite")
                }
            })?;

        let table_ids: Vec<_> = container_ids
            .iter()
            .filter_map(|(name, id)| id.left().map(|id| (name.as_str(), id)))
            .collect();

        let truncate_sql = table_ids
            .iter()
            .map(|(table_name, _)| format!("DELETE FROM {}", table_name))
            .collect::<Vec<_>>()
            .join("; ");

        // Avoid executing a query if there are no tables to truncate.
        // This prevents a "No SQL statements detected" error.
        if table_ids.is_empty() {
            return Ok(());
        }

        tracing::debug!(query_type = "reset", truncate_sql);

        self.adapter
            .execute_script(&format!(
                r#"
                    PRAGMA defer_foreign_keys = 1;
                    {}
                "#,
                truncate_sql.as_str(),
            ))
            .await
            .map_err(convert_error)
    }

    async fn dispose(&self) -> ConnectorResult<()> {
        self.adapter.dispose().await.map_err(convert_error)
    }
}

pub fn connect_to_shadow_db() -> ConnectorResult<Connection> {
    Err(ConnectorError::from_msg(
        "SQLite shadow DB must be provided through an external factory".to_owned(),
    ))
}

pub async fn create_database(_state: &State) -> ConnectorResult<String> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub async fn drop_database(_state: &State) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub async fn ensure_connection_validity(state: &mut State) -> ConnectorResult<()> {
    let (connection, _) = get_connection_and_params(state)?;
    connection.version().await?;
    Ok(())
}

pub async fn introspect(state: &mut State) -> ConnectorResult<SqlSchema> {
    super::describe_schema(&state.connection).await
}

pub fn get_connection_and_params(state: &mut State) -> ConnectorResult<(&Connection, &Params)> {
    Ok((&state.connection, &Params))
}

pub fn set_preview_features(state: &mut State, features: BitFlags<PreviewFeature>) {
    state.preview_features = features;
}

pub fn get_preview_features(state: &State) -> psl::PreviewFeatures {
    state.preview_features
}

pub fn get_shadow_db_url(_state: &State) -> Option<&str> {
    None
}

pub async fn dispose(state: &State) -> ConnectorResult<()> {
    state.connection.dispose().await
}

fn convert_error(err: quaint::error::Error) -> ConnectorError {
    match err.kind() {
        quaint::error::ErrorKind::ExternalError(id) => {
            ConnectorError::user_facing(user_facing_errors::query_engine::ExternalError { id: *id })
        }
        _ => ConnectorError::from_source(err, "Error executing SQLite query."),
    }
}

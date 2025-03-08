//! All the quaint-wrangling for the sqlite connector should happen here.

use crate::BitFlags;
use crate::ConnectorParams;
use psl::PreviewFeature;
use quaint::connector::ExternalConnector;
use quaint::connector::ExternalConnectorFactory;
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::SqlSchema;
use std::sync::Arc;

pub(super) struct State {
    connection: Connection,
    factory: Arc<dyn ExternalConnectorFactory>,
    preview_features: BitFlags<PreviewFeature>,
}

impl State {
    pub fn new(
        adapter: Arc<dyn ExternalConnector>,
        factory: Arc<dyn ExternalConnectorFactory>,
        preview_features: BitFlags<PreviewFeature>,
    ) -> Self {
        Self {
            preview_features,
            factory,
            connection: Connection { adapter },
        }
    }
}

pub(super) struct Params;

pub(super) struct Connection {
    adapter: Arc<dyn ExternalConnector>,
}

impl Connection {
    pub fn as_connector(&self) -> &Arc<dyn ExternalConnector> {
        &self.adapter
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
        self.adapter.version().await.map_err(convert_error)
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
        self.adapter
            .execute_script(
                r#"
            PRAGMA writable_schema = 1;
            DELETE FROM sqlite_master;
            PRAGMA writable_schema = 0;
            VACUUM;
            PRAGMA integrity_check;
            "#,
            )
            .await
            .map_err(convert_error)
    }

    pub async fn dispose(&self) -> ConnectorResult<()> {
        self.adapter.dispose().await.map_err(convert_error)
    }
}

pub async fn new_shadow_db(state: &State) -> ConnectorResult<Connection> {
    let adapter = state
        .factory
        .connect_to_shadow_db()
        .await
        .ok_or_else(|| ConnectorError::from_msg("Invalid SQLite adapter: missing connectToShadowDb".to_owned()))?
        .map_err(convert_error)?;
    Ok(Connection { adapter })
}

pub(super) async fn create_database(_state: &State) -> ConnectorResult<String> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) async fn drop_database(_state: &State) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) async fn ensure_connection_validity(state: &mut State) -> ConnectorResult<()> {
    let (connection, _) = get_connection_and_params(state)?;
    connection.version().await?;
    Ok(())
}

pub(super) async fn introspect(state: &mut State) -> ConnectorResult<SqlSchema> {
    super::describe_schema(&state.connection).await
}

pub(super) fn get_connection_string(_state: &State) -> Option<&str> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn get_connection_and_params(state: &mut State) -> ConnectorResult<(&Connection, &Params)> {
    Ok((&state.connection, &Params))
}

pub(super) fn set_params(_state: &mut State, _params: ConnectorParams) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn set_preview_features(state: &mut State, features: BitFlags<PreviewFeature>) {
    state.preview_features = features;
}

fn convert_error(err: quaint::error::Error) -> ConnectorError {
    ConnectorError::from_source(err, "external connector error")
}

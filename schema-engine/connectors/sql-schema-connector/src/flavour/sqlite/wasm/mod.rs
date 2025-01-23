//! All the quaint-wrangling for the sqlite connector should happen here.

use crate::BitFlags;
use crate::ConnectorParams;
use psl::PreviewFeature;
use quaint::connector::ExternalConnector;
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::SqlSchema;
use std::sync::Arc;

pub(super) struct State {
    connection: Connection,
    preview_features: BitFlags<PreviewFeature>,
}

impl State {
    pub fn new(connection: Arc<dyn ExternalConnector>, preview_features: BitFlags<PreviewFeature>) -> Self {
        Self {
            preview_features,
            connection: Connection(connection),
        }
    }
}

pub(super) struct Connection(Arc<dyn ExternalConnector>);

impl Connection {
    pub fn new_in_memory() -> Self {
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }

    pub fn as_connector(&self) -> &Arc<dyn ExternalConnector> {
        &self.0
    }

    pub async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await.map_err(convert_error)
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
        self.0.query_raw(sql, params).await.map_err(convert_error)
    }

    pub async fn apply_migration_script(&self, _migration_name: &str, _script: &str) -> ConnectorResult<()> {
        panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
    }

    pub async fn version(&self) -> ConnectorResult<Option<String>> {
        self.0.version().await.map_err(convert_error)
    }
}

pub(super) fn create_database(_params: &State) -> ConnectorResult<String> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn drop_database(_state: &State) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) async fn reset(_state: &mut State) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) async fn ensure_connection_validity(state: &State) -> ConnectorResult<()> {
    state.connection.version().await?;
    Ok(())
}

pub(super) async fn introspect(instance: &mut State) -> ConnectorResult<SqlSchema> {
    super::describe_schema(&instance.connection).await
}

pub(super) fn describe_query(state: &mut State, sql: &str) -> ConnectorResult<quaint::connector::DescribedQuery> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn connection_string(_state: &State) -> Option<&str> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

pub(super) fn get_connection(state: &mut State) -> ConnectorResult<&mut Connection> {
    Ok(&mut state.connection)
}

pub(super) fn set_preview_features(state: &mut State, features: BitFlags<PreviewFeature>) {
    state.preview_features = features;
}

pub(super) fn set_params(_state: &mut State, params: ConnectorParams) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::sqlite::wasm] Not implemented");
}

fn convert_error(err: quaint::error::Error) -> ConnectorError {
    ConnectorError::from_source(err, "external connector error")
}

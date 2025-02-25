//! All the quaint-wrangling for the postgres connector should happen here.

pub(super) mod shadow_db;

use crate::flavour::postgres::{Circumstances, PostgresProvider, ADVISORY_LOCK_TIMEOUT};
use crate::{BitFlags, ConnectorParams};
use psl::PreviewFeature;
use quaint::connector::ExternalConnector;
use schema_connector::{ConnectorError, ConnectorResult};
use std::sync::Arc;

pub(super) struct State {
    connection: Connection,
    circumstances: BitFlags<Circumstances>,
    preview_features: BitFlags<PreviewFeature>,
}

pub(super) struct Params;

impl State {
    pub async fn new(
        connection: Arc<dyn ExternalConnector>,
        provider: PostgresProvider,
        preview_features: BitFlags<PreviewFeature>,
    ) -> ConnectorResult<Self> {
        let connection = Connection(connection);
        // TODO: we don't have a URL for the adapter, this defaults schema to "public", we might
        // want to do something more sophisticated here.
        let circumstances = super::setup_connection(&connection, &Params, provider, "public").await?;
        Ok(Self {
            connection,
            circumstances,
            preview_features,
        })
    }
}

pub(super) struct Connection(Arc<dyn ExternalConnector>);

impl Connection {
    pub fn as_connector(&self) -> &Arc<dyn ExternalConnector> {
        &self.0
    }

    // Query methods return quaint::Result directly to let the caller decide how to convert
    // the error. This is needed for errors that use information related to the connection.

    pub async fn raw_cmd(&self, sql: &str) -> quaint::Result<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.0.raw_cmd(sql).await
    }

    pub async fn query(&self, query: quaint::ast::Query<'_>) -> quaint::Result<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Postgres::build(query).unwrap();
        self.query_raw(&sql, &params).await
    }

    pub async fn query_raw(
        &self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
    ) -> quaint::Result<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        self.0.query_raw(sql, params).await
    }

    pub async fn version(&self) -> quaint::Result<Option<String>> {
        self.0.version().await
    }

    pub async fn describe_query(&self, sql: &str) -> quaint::Result<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        self.0.describe_query(sql).await
    }

    pub async fn apply_migration_script(&self, _migration_name: &str, script: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "apply_migration_script", script);
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }
}

pub(super) async fn create_database(state: &State) -> ConnectorResult<String> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) async fn drop_database(state: &State) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) fn get_connection_string(_state: &State) -> Option<&str> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) fn get_circumstances(state: &State) -> Option<BitFlags<Circumstances>> {
    Some(state.circumstances)
}

pub(super) fn get_default_schema(_params: &State) -> Option<&'static str> {
    None
}

pub(super) async fn get_connection_and_params_and_circumstances(
    state: &mut State,
    _provider: PostgresProvider,
) -> ConnectorResult<(&Connection, &Params, BitFlags<Circumstances>)> {
    Ok((&state.connection, &Params, state.circumstances))
}

pub(super) async fn get_connection_and_params(
    state: &mut State,
    _provider: PostgresProvider,
) -> ConnectorResult<(&Connection, &Params)> {
    Ok((&state.connection, &Params))
}

pub(super) fn set_params(_state: &mut State, _params: ConnectorParams) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) fn get_preview_features(state: &State) -> BitFlags<PreviewFeature> {
    state.preview_features
}

pub(super) fn set_preview_features(state: &mut State, features: BitFlags<PreviewFeature>) {
    state.preview_features = features;
}

pub(super) fn quaint_error_mapper(_params: &Params) -> impl Fn(quaint::error::Error) -> ConnectorError {
    |err| ConnectorError::from_source(err, "external connector error")
}

pub(super) fn timeout_error(_params: &Params) -> ConnectorError {
    ConnectorError::user_facing(user_facing_errors::common::DatabaseTimeout {
        database_host: "<driver-adapter-host>".to_string(),
        database_port: "<driver-adapter-port>".to_string(),
        context: format!(
            "Timed out trying to acquire a postgres advisory lock (SELECT pg_advisory_lock(72707369)). Elapsed: {}ms. See https://pris.ly/d/migrate-advisory-locking for details.", ADVISORY_LOCK_TIMEOUT.as_millis()
        ),
    })
}

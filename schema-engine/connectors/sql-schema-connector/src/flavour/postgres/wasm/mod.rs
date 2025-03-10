//! All the quaint-wrangling for the postgres connector should happen here.

pub(super) mod shadow_db;

use crate::flavour::postgres::{Circumstances, PostgresProvider, ADVISORY_LOCK_TIMEOUT};
use crate::{BitFlags, ConnectorParams};
use psl::PreviewFeature;
use quaint::connector::{ExternalConnector, ExternalConnectorFactory};
use schema_connector::{ConnectorError, ConnectorResult};
use std::sync::Arc;

pub(super) struct State {
    connection: Connection,
    factory: Arc<dyn ExternalConnectorFactory>,
    schema_name: String,
    circumstances: BitFlags<Circumstances>,
    preview_features: BitFlags<PreviewFeature>,
}

pub(super) struct Params;

impl State {
    pub async fn new(
        adapter: Arc<dyn ExternalConnector>,
        factory: Arc<dyn ExternalConnectorFactory>,
        provider: PostgresProvider,
        preview_features: BitFlags<PreviewFeature>,
    ) -> ConnectorResult<Self> {
        let info = adapter
            .get_connection_info()
            .await
            .map_err(|err| ConnectorError::from_source(err, "failed to get connection info"))?;
        let schema_name = info.schema_name.to_owned();

        let connection = Connection { adapter };
        let circumstances = super::setup_connection(&connection, &Params, provider, &schema_name).await?;
        Ok(Self {
            connection,
            factory,
            schema_name,
            circumstances,
            preview_features,
        })
    }

    pub async fn new_shadow_db(&self) -> ConnectorResult<Connection> {
        let adapter = self
            .factory
            .connect_to_shadow_db()
            .await
            .ok_or_else(|| ConnectorError::from_msg("Invalid Postgres adapter: missing connectToShadowDb".to_owned()))?
            .map_err(|err| quaint_error_mapper(&Params)(err).into_shadow_db_creation_error())?;
        Ok(Connection { adapter })
    }
}

pub(super) struct Connection {
    adapter: Arc<dyn ExternalConnector>,
}

impl Connection {
    pub fn as_connector(&self) -> &Arc<dyn ExternalConnector> {
        &self.adapter
    }

    // Query methods return quaint::Result directly to let the caller decide how to convert
    // the error. This is needed for errors that use information related to the connection.

    pub async fn raw_cmd(&self, sql: &str) -> quaint::Result<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.adapter.raw_cmd(sql).await
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
        self.adapter.query_raw(sql, params).await
    }

    pub async fn version(&self) -> quaint::Result<Option<String>> {
        self.adapter.version().await
    }

    pub async fn describe_query(&self, sql: &str) -> quaint::Result<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        self.adapter.describe_query(sql).await
    }

    pub async fn apply_migration_script(&self, _migration_name: &str, script: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "apply_migration_script", script);
        self.adapter
            .execute_script(script)
            .await
            .map_err(|err| ConnectorError::from_source(err, "external connector error"))
    }

    pub async fn dispose(&self) -> quaint::Result<()> {
        self.adapter.dispose().await
    }
}

pub(super) async fn create_database(_state: &State) -> ConnectorResult<String> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) async fn drop_database(_state: &State) -> ConnectorResult<()> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) fn get_connection_string(_state: &State) -> Option<&str> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

pub(super) fn get_circumstances(state: &State) -> Option<BitFlags<Circumstances>> {
    Some(state.circumstances)
}

pub(super) fn get_default_schema(state: &State) -> &str {
    &state.schema_name
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

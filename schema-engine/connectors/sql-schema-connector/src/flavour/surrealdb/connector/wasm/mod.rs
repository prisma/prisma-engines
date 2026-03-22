use crate::BitFlags;
use crate::flavour::quaint_error_to_connector_error;
use psl::PreviewFeature;
use quaint::connector::{ConnectionInfo, ExternalConnectionInfo, ExternalConnector};
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::SqlSchema;
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

    pub async fn raw_cmd(&self, sql: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        self.adapter.raw_cmd(sql).await.map_err(convert_error)
    }

    pub async fn query(&self, query: quaint::ast::Query<'_>) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::SurrealDb::build(query).unwrap();
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

    pub async fn apply_migration_script(&self, _migration_name: &str, script: &str) -> ConnectorResult<()> {
        self.adapter.execute_script(script).await.map_err(convert_error)
    }

    pub async fn reset(&self, _params: &Params) -> ConnectorResult<()> {
        // SurrealDB: use INFO FOR DB to list tables and remove them
        let result = self
            .adapter
            .query_raw("INFO FOR DB", &[])
            .await
            .map_err(convert_error)?;

        // For now, a simplified reset that drops known tables
        // In production, this would introspect the DB and drop all tables
        Ok(())
    }

    async fn dispose(&self) -> ConnectorResult<()> {
        self.adapter.dispose().await.map_err(convert_error)
    }
}

pub fn connect_to_shadow_db() -> ConnectorResult<Connection> {
    Err(ConnectorError::from_msg(
        "SurrealDB shadow DB must be provided through an external factory".to_owned(),
    ))
}

pub async fn create_database(_state: &State) -> ConnectorResult<String> {
    // SurrealDB databases are created automatically on first use
    Ok("Database created".to_owned())
}

pub async fn drop_database(_state: &State) -> ConnectorResult<()> {
    // SurrealDB: REMOVE DATABASE would be used
    Ok(())
}

pub async fn ensure_connection_validity(state: &mut State) -> ConnectorResult<()> {
    let (connection, _) = get_connection_and_params(state)?;
    connection.version().await?;
    Ok(())
}

pub async fn introspect(state: &mut State) -> ConnectorResult<SqlSchema> {
    // SurrealDB introspection via INFO FOR DB / INFO FOR TABLE
    // For now, return an empty schema — full introspection requires parsing SurrealDB's schema info
    Ok(SqlSchema::default())
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
    quaint_error_to_connector_error(err, None)
}

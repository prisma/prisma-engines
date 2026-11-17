use super::Params;
use crate::flavour::quaint_error_to_connector_error;
use quaint::{
    connector::{KingbaseOracle, KingbaseOracleUrl},
    prelude::{NativeConnectionInfo, Queryable},
};
use schema_connector::{ConnectorError, ConnectorResult};
use sql_schema_describer::{DescriberErrorKind, SqlSchema, SqlSchemaDescriberBackend};
use url::Url;
use user_facing_errors::schema_engine::{ApplyMigrationError, DatabaseSchemaInconsistent};

pub(super) struct Connection {
    client: KingbaseOracle,
    url: KingbaseOracleUrl,
}

impl Connection {
    pub(super) async fn new(url: Url) -> ConnectorResult<Self> {
        let url = KingbaseOracleUrl::new(url).map_err(ConnectorError::url_parse_error)?;
        let connection = KingbaseOracle::new(url.clone()).await.map_err(|error| {
            quaint_error_to_connector_error(error, Some(&NativeConnectionInfo::KingbaseOracle(url.clone())))
        })?;

        Ok(Self {
            client: connection,
            url,
        })
    }

    pub(super) async fn describe_schema(
        &mut self,
        params: &Params,
        namespaces: Option<schema_connector::Namespaces>,
    ) -> ConnectorResult<SqlSchema> {
        let mut schemas = namespaces
            .map(|namespaces| namespaces.into_iter().collect::<Vec<_>>())
            .unwrap_or_default();

        if schemas.is_empty() {
            schemas.push(params.schema_name().to_owned());
        }

        let schema_refs = schemas.iter().map(String::as_str).collect::<Vec<_>>();
        let schema = sql_schema_describer::kingbase_oracle::SqlSchemaDescriber::new(&self.client)
            .describe(&schema_refs)
            .await
            .map_err(|error| match error.into_kind() {
                DescriberErrorKind::QuaintError(error) => self.map_error(error),
                error @ DescriberErrorKind::CrossSchemaReference { .. } => {
                    ConnectorError::user_facing(DatabaseSchemaInconsistent {
                        explanation: error.to_string(),
                    })
                }
            })?;

        Ok(schema)
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str) -> ConnectorResult<()> {
        self.client.raw_cmd(sql).await.map_err(|error| self.map_error(error))
    }

    pub(super) async fn version(&mut self) -> ConnectorResult<Option<String>> {
        self.client.version().await.map_err(|error| self.map_error(error))
    }

    pub(super) async fn query(&mut self, query: quaint::ast::Query<'_>) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.client.query(query).await.map_err(|error| self.map_error(error))
    }

    pub(super) async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::Value<'_>],
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        self.client
            .query_raw(sql, params)
            .await
            .map_err(|error| self.map_error(error))
    }

    pub(super) async fn describe_query(&mut self, sql: &str) -> ConnectorResult<quaint::connector::DescribedQuery> {
        self.client
            .describe_query(sql)
            .await
            .map_err(|error| self.map_error(error))
    }

    pub(super) async fn apply_migration_script(&mut self, migration_name: &str, script: &str) -> ConnectorResult<()> {
        self.client.raw_cmd(script).await.map_err(|error| {
            let database_error_code = error.original_code().unwrap_or("none").to_owned();
            let database_error = error
                .original_message()
                .map(str::to_owned)
                .unwrap_or_else(|| error.to_string());

            ConnectorError::user_facing(ApplyMigrationError {
                migration_name: migration_name.to_owned(),
                database_error_code,
                database_error,
            })
        })
    }

    fn map_error(&self, error: quaint::error::Error) -> ConnectorError {
        let connection_info = NativeConnectionInfo::KingbaseOracle(self.url.clone());
        quaint_error_to_connector_error(error, Some(&connection_info))
    }
}

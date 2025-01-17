//! All the quaint-wrangling for the postgres connector should happen here.

pub(super) mod shadow_db;

use enumflags2::BitFlags;
use quaint::connector::{ColumnType, DescribedColumn, DescribedParameter, GetRow, ToColumnNames};
use schema_connector::{BoxFuture, ConnectorError, ConnectorResult, Namespaces};
use sql_schema_describer::{postgres as describer, DescriberErrorKind, SqlSchema};
use user_facing_errors::schema_engine::ApplyMigrationError;

use super::MigratePostgresUrl;

// TODO: use ExternalConnector here.
pub(super) struct Connection();

impl Connection {
    pub(super) async fn new(url: url::Url) -> ConnectorResult<Self> {
        // TODO: establish a connection, gather the version string,
        // determine whether it is a CockroachDB instance or not.
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }

    #[tracing::instrument(skip(self, circumstances, params))]
    pub(super) async fn describe_schema(
        &mut self,
        circumstances: BitFlags<super::Circumstances>,
        params: &super::Params,
        namespaces: Option<Namespaces>,
    ) -> ConnectorResult<SqlSchema> {
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, _url: &MigratePostgresUrl) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }

    pub(super) async fn version(&mut self, url: &MigratePostgresUrl) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }

    pub(super) async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        url: &MigratePostgresUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Postgres::build(query).unwrap();
        self.query_raw(&sql, &params, url).await
    }

    pub(super) async fn query_raw(
        &mut self,
        sql: &str,
        params: &[quaint::prelude::Value<'_>],
        _url: &MigratePostgresUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }

    pub(super) async fn describe_query(
        &mut self,
        sql: &str,
        _url: &MigratePostgresUrl,
    ) -> ConnectorResult<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }

    pub(super) async fn apply_migration_script(&mut self, migration_name: &str, script: &str) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", script);
        panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
    }
}

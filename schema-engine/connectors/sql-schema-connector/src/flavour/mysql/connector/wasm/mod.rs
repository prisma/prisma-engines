//! All the quaint-wrangling for the mysql connector should happen here.
#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

pub(super) mod shadow_db;

use enumflags2::BitFlags;
use quaint::connector::{ColumnType, DescribedColumn, DescribedParameter, GetRow, MysqlUrl, ToColumnNames};
use schema_connector::{BoxFuture, ConnectorError, ConnectorResult};
use sql_schema_describer::{mysql as describer, DescriberErrorKind, SqlSchema};
use user_facing_errors::schema_engine::ApplyMigrationError;

// TODO: use ExternalConnector here.
pub struct Connection();

impl Connection {
    pub(super) async fn new(_url: url::Url) -> ConnectorResult<Self> {
        // TODO: establish a connection, gather the version string,
        // determine whether it is a CockroachDB instance or not.
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }

    #[tracing::instrument(skip(self, circumstances, params))]
    pub(super) async fn describe_schema(
        &mut self,
        _circumstances: BitFlags<super::Circumstances>,
        _params: &super::Params,
    ) -> ConnectorResult<SqlSchema> {
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }

    pub(super) async fn raw_cmd(&mut self, sql: &str, _url: &MysqlUrl) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", sql);
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }

    pub(super) async fn version(&mut self, _url: &MysqlUrl) -> ConnectorResult<Option<String>> {
        tracing::debug!(query_type = "version");
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }

    pub(super) async fn query(
        &mut self,
        query: quaint::ast::Query<'_>,
        url: &MysqlUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        use quaint::visitor::Visitor;
        let (sql, params) = quaint::visitor::Mysql::build(query).unwrap();
        self.query_raw(&sql, &params, url).await
    }

    pub(super) async fn query_raw(
        &mut self,
        sql: &str,
        _params: &[quaint::prelude::Value<'_>],
        _url: &MysqlUrl,
    ) -> ConnectorResult<quaint::prelude::ResultSet> {
        tracing::debug!(query_type = "query_raw", sql);
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }

    pub(super) async fn describe_query(
        &mut self,
        sql: &str,
        _url: &MysqlUrl,
        _circumstances: BitFlags<super::Circumstances>,
    ) -> ConnectorResult<quaint::connector::DescribedQuery> {
        tracing::debug!(query_type = "describe_query", sql);
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }

    pub(super) async fn apply_migration_script(
        &mut self,
        _migration_name: &str,
        _script: &str,
        _circumstances: BitFlags<super::Circumstances>,
    ) -> ConnectorResult<()> {
        tracing::debug!(query_type = "raw_cmd", script);
        panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
    }
}

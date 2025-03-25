#![cfg_attr(target_arch = "wasm32", allow(unused_imports))]

use crate::flavour::{MysqlConnector, SqlConnector};
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    _migrations: &[MigrationDirectory],
    _shadow_db: &mut MysqlConnector,
) -> ConnectorResult<SqlSchema> {
    panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
}

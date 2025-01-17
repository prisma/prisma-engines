use crate::flavour::{MysqlFlavour, SqlFlavour};
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    migrations: &[MigrationDirectory],
    mut shadow_db: MysqlFlavour,
) -> ConnectorResult<SqlSchema> {
    panic!("[sql-schema-connector::flavour::mysql::wasm] Not implemented");
}

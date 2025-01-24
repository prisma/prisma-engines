use crate::flavour::postgres::PostgresProvider;
use schema_connector::Namespaces;
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migration_history(
    _state: &mut super::State,
    _provider: PostgresProvider,
    _migrations: &[MigrationDirectory],
    _shadow_database_connection_string: Option<String>,
    _namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

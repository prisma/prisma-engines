use crate::flavour::{PostgresFlavour, SqlFlavour};
use schema_connector::Namespaces;
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    migrations: &[MigrationDirectory],
    mut shadow_db: PostgresFlavour,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    panic!("[sql-schema-connector::flavour::postgres::wasm] Not implemented");
}

use crate::flavour::{MssqlFlavour, SqlFlavour};
use schema_connector::Namespaces;
use schema_connector::{ConnectorResult, migrations_directory::MigrationDirectory};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migrations_history(
    migrations: &MigrationDirectories,
    mut shadow_db: MssqlFlavour,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    panic!("[sql-schema-connector::flavour::mssql::wasm] Not implemented");
}

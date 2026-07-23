use crate::flavour::postgres::{
    PostgresConnector, PostgresProvider, UsingExternalShadowDb, sql_schema_from_migrations_and_db,
};
use schema_connector::{ConnectorError, ConnectorResult, Namespaces, SchemaFilter, migrations_directory::Migrations};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migration_history(
    connector: &mut PostgresConnector,
    _provider: PostgresProvider,
    migrations: &Migrations,
    namespaces: Option<Namespaces>,
    _filter: &SchemaFilter,
    external_shadow_db: UsingExternalShadowDb,
) -> ConnectorResult<SqlSchema> {
    let schema = connector.schema_name().to_owned();
    let circumstances = connector.state.circumstances;
    let preview_features = connector.state.preview_features;

    if matches!(external_shadow_db, UsingExternalShadowDb::No) {
        return Err(ConnectorError::from_msg(
            "PostgreSQL shadow DB must be provided through an external factory".to_owned(),
        ));
    }

    // TODO: should we do a best effort reset here similar to in sql_schema_from_migration_history_for_external_db?

    connector
        .with_connection(|conn, params| {
            sql_schema_from_migrations_and_db(
                conn,
                params,
                schema,
                migrations,
                namespaces,
                circumstances,
                preview_features,
            )
        })
        .await
}

use crate::flavour::postgres::{sql_schema_from_migrations_and_db, PostgresConnector, PostgresProvider};
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use schema_connector::{ConnectorError, Namespaces, UsingExternalShadowDb};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migration_history(
    connector: &mut PostgresConnector,
    _provider: PostgresProvider,
    migrations: &[MigrationDirectory],
    namespaces: Option<Namespaces>,
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

    connector
        .with_connection(|conn, params| {
            sql_schema_from_migrations_and_db(
                &conn,
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

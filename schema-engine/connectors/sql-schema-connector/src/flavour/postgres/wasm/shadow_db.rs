use crate::flavour::postgres::{sql_schema_from_migrations_and_db, PostgresProvider};
use schema_connector::Namespaces;
use schema_connector::{migrations_directory::MigrationDirectory, ConnectorResult};
use sql_schema_describer::SqlSchema;

pub async fn sql_schema_from_migration_history(
    state: &mut super::State,
    _provider: PostgresProvider,
    migrations: &[MigrationDirectory],
    _shadow_database_connection_string: Option<String>,
    namespaces: Option<Namespaces>,
) -> ConnectorResult<SqlSchema> {
    let conn = state.new_shadow_db().await?;

    let schema = super::get_default_schema(state).to_owned();
    let result = sql_schema_from_migrations_and_db(
        &conn,
        &super::Params,
        schema,
        migrations,
        namespaces,
        state.circumstances,
        state.preview_features,
    )
    .await;
    // dispose the shadow database connection regardless of the result
    conn.dispose()
        .await
        .map_err(super::quaint_error_mapper(&super::Params))?;

    result
}

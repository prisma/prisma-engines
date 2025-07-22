use crate::{CoreResult, SchemaContainerExt, json_rpc::types::*, parse_schema_multi};
use schema_connector::{ConnectorError, SchemaConnector};
use tracing_futures::Instrument;

/// Command to bring the local database in sync with the prisma schema, without
/// interacting with the migrations directory nor the migrations table.
pub async fn schema_push(input: SchemaPushInput, connector: &mut dyn SchemaConnector) -> CoreResult<SchemaPushOutput> {
    let sources = input.schema.to_psl_input();
    let datamodel = parse_schema_multi(&sources)?;

    if let Some(err) = connector.check_database_version_compatibility(&datamodel) {
        return Err(ConnectorError::user_facing(err));
    };

    // The `ensure_connection_validity` call is currently needed to infer the correct
    // circumstances from the connector. This is necessary because otherwise the state machine
    // we use for native drivers doesn't get initialized and some features like CockroachDB
    // detection do not work.
    // The error is intentionally ignored because it interferes with some tests ('schemaPush ›
    // should succeed if SQLite database file is missing' in prisma/prisma).
    //
    // TODO: We should remove this call once the state machines are no longer used.
    let _ = connector.ensure_connection_validity().await;
    let dialect = connector.schema_dialect();
    let filter: schema_connector::SchemaFilter = input.filters.into();

    let to = dialect.schema_from_datamodel(sources)?;
    // We only consider the namespaces present in the "to" schema aka the PSL file for the introspection of the "from" schema.
    // So when the user removes a previously existing namespace from their PSL file we will not modify that namespace in the database.
    let namespaces = dialect.extract_namespaces(&to);
    filter.validate(namespaces.as_ref())?;

    let from = connector
        .schema_from_database(namespaces)
        .instrument(tracing::info_span!("Calculate from database"))
        .await?;
    let database_migration = dialect.diff(from, to, &filter);

    tracing::debug!(migration = dialect.migration_summary(&database_migration).as_str());

    let checks = connector
        .destructive_change_checker()
        .check(&database_migration)
        .await?;

    let executed_steps = match (checks.unexecutable_migrations.len(), checks.warnings.len(), input.force) {
        (unexecutable, _, _) if unexecutable > 0 => {
            tracing::warn!(unexecutable = ?checks.unexecutable_migrations, "Aborting migration because at least one unexecutable step was detected.");

            0
        }
        (0, 0, _) | (0, _, true) => connector.apply_migration(&database_migration).await?,
        _ => {
            tracing::info!(
                "The migration was not applied because it triggered warnings and the force flag was not passed."
            );

            0
        }
    };

    let warnings = checks.warnings.into_iter().map(|warning| warning.description).collect();

    let unexecutable = checks
        .unexecutable_migrations
        .into_iter()
        .map(|unexecutable| unexecutable.description)
        .collect();

    Ok(SchemaPushOutput {
        executed_steps,
        warnings,
        unexecutable,
    })
}

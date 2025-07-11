use super::{
    diagnose_migration_history_cli, DiagnoseMigrationHistoryOutput, DriftDiagnostic, HistoryDiagnostic,
    MigrationSchemaCache,
};
use crate::json_rpc::types::{
    DevAction, DevActionReset, DevDiagnosticInput, DevDiagnosticOutput, DiagnoseMigrationHistoryInput,
};
use schema_connector::{migrations_directory, ConnectorResult, Namespaces, SchemaConnector};

/// Method called at the beginning of `migrate dev` to decide the course of
/// action based on the current state of the workspace.
pub async fn dev_diagnostic_cli(
    input: DevDiagnosticInput,
    namespaces: Option<Namespaces>,
    connector: &mut dyn SchemaConnector,
    migration_schema_cache: &mut MigrationSchemaCache,
) -> ConnectorResult<DevDiagnosticOutput> {
    migrations_directory::error_on_changed_provider(&input.migrations_list.lockfile, connector.connector_type())?;

    let diagnose_input = DiagnoseMigrationHistoryInput {
        migrations_list: input.migrations_list,
        opt_in_to_shadow_database: true,
        schema_filter: input.schema_filter,
    };

    let diagnose_migration_history_output =
        diagnose_migration_history_cli(diagnose_input, namespaces, connector, migration_schema_cache).await?;

    check_for_broken_migrations(&diagnose_migration_history_output)?;

    if let Some(reason) = check_for_reset_conditions(&diagnose_migration_history_output) {
        return Ok(DevDiagnosticOutput {
            action: DevAction::Reset(DevActionReset { reason }),
        });
    }

    Ok(DevDiagnosticOutput {
        action: DevAction::CreateMigration,
    })
}

fn check_for_broken_migrations(output: &DiagnoseMigrationHistoryOutput) -> ConnectorResult<()> {
    if let Some(DriftDiagnostic::MigrationFailedToApply { error }) = &output.drift {
        return Err(error.clone());
    }

    if let Some(error) = &output.error_in_unapplied_migration {
        return Err(error.clone());
    }

    Ok(())
}

fn check_for_reset_conditions(output: &DiagnoseMigrationHistoryOutput) -> Option<String> {
    let mut reset_reasons = Vec::new();

    for failed_migration_name in &output.failed_migration_names {
        reset_reasons.push(format!("The migration `{failed_migration_name}` failed."));
    }

    for edited_migration_name in &output.edited_migration_names {
        reset_reasons.push(format!(
            "The migration `{edited_migration_name}` was modified after it was applied."
        ))
    }

    if let Some(DriftDiagnostic::DriftDetected { summary }) = &output.drift {
        let mut reason = DRIFT_DETECTED_MESSAGE.trim_start().to_owned();

        if !output.has_migrations_table {
            reason.push_str(FIRST_TIME_MIGRATION_MESSAGE);
        }

        reason.push_str(summary);
        reset_reasons.push(reason);
    }

    match &output.history {
        Some(HistoryDiagnostic::HistoriesDiverge { last_common_migration_name, unapplied_migration_names: _, unpersisted_migration_names }) => {
            let details = last_common_migration_name.as_ref().map(|last_common_migration_name|{
                format!(" Last common migration: `{}`. Migrations applied to the database but absent from the migrations directory are: {}", last_common_migration_name, unpersisted_migration_names.join(", "))
            }).unwrap_or_else(String::new);

            reset_reasons.push(format!("The migrations recorded in the database diverge from the local migrations directory.{details}"))
        },
        Some(HistoryDiagnostic::MigrationsDirectoryIsBehind { unpersisted_migration_names}) => reset_reasons.push(
           format!("The following migration(s) are applied to the database but missing from the local migrations directory: {}", unpersisted_migration_names.join(", ")),
        ),
        None | Some(HistoryDiagnostic::DatabaseIsBehind { .. }) => (),
    }

    match reset_reasons.as_slice() {
        [] => None,
        [first_reason] => Some(first_reason.clone()),
        _ => {
            let mut message = String::with_capacity(reset_reasons.iter().map(|s| s.len() + 3).sum::<usize>());

            for reason in reset_reasons {
                message.push_str("- ");
                message.push_str(&reason);
                message.push('\n');
            }

            Some(message)
        }
    }
}

const DRIFT_DETECTED_MESSAGE: &str = r#"
Drift detected: Your database schema is not in sync with your migration history.

The following is a summary of the differences between the expected database schema given your migrations files, and the actual schema of the database.

It should be understood as the set of changes to get from the expected schema to the actual schema.
"#;

const FIRST_TIME_MIGRATION_MESSAGE: &str = r#"
If you are running this the first time on an existing database, please make sure to read this documentation page:
https://www.prisma.io/docs/guides/database/developing-with-prisma-migrate/troubleshooting-development
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dev_action_serializes_as_expected() {
        let reset = serde_json::to_value(DevAction::Reset(DevActionReset {
            reason: "Because I said so".to_owned(),
        }))
        .unwrap();

        assert_eq!(reset, json!({ "tag": "reset", "reason": "Because I said so" }));

        let create_migration = serde_json::to_value(DevAction::CreateMigration).unwrap();

        assert_eq!(create_migration, json!({ "tag": "createMigration" }));
    }
}

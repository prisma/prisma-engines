use super::{
    DiagnoseMigrationHistoryCommand, DiagnoseMigrationHistoryInput, DiagnoseMigrationHistoryOutput, DriftDiagnostic,
    HistoryDiagnostic, MigrationCommand,
};
use crate::core_error::CoreResult;
use migration_connector::MigrationConnector;
use serde::{Deserialize, Serialize};

/// Method called at the beginning of `migrate dev` to decide the course of
/// action based on the current state of the workspace.
pub struct DevDiagnosticCommand;

/// The `devDiagnostic` input.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DevDiagnosticInput {
    /// The location of the migrations directory.
    pub migrations_directory_path: String,
}

/// The response type for `devDiagnostic`.
#[derive(Debug, Serialize)]
pub struct DevDiagnosticOutput {
    /// The suggested course of action for the CLI.
    pub action: DevAction,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for DevDiagnosticCommand {
    type Input = DevDiagnosticInput;
    type Output = DevDiagnosticOutput;

    async fn execute<C: MigrationConnector>(input: &Self::Input, connector: &C) -> CoreResult<Self::Output> {
        migration_connector::error_on_changed_provider(&input.migrations_directory_path, connector.connector_type())?;

        let diagnose_input = DiagnoseMigrationHistoryInput {
            migrations_directory_path: input.migrations_directory_path.clone(),
            opt_in_to_shadow_database: true,
        };

        let mut diagnose_migration_history_output =
            DiagnoseMigrationHistoryCommand::execute(&diagnose_input, connector).await?;

        check_for_broken_migrations(&mut diagnose_migration_history_output)?;

        if let Some(reason) = check_for_reset_conditions(&diagnose_migration_history_output) {
            return Ok(DevDiagnosticOutput {
                action: DevAction::Reset { reason },
            });
        }

        Ok(DevDiagnosticOutput {
            action: DevAction::CreateMigration,
        })
    }
}

fn check_for_broken_migrations(output: &mut DiagnoseMigrationHistoryOutput) -> CoreResult<()> {
    if let Some(drift) = output.drift.take() {
        match drift {
            DriftDiagnostic::MigrationFailedToApply { error } => return Err(error),
            _ => output.drift = Some(drift),
        }
    }

    if let Some(error) = output.error_in_unapplied_migration.take() {
        return Err(error);
    }

    Ok(())
}

fn check_for_reset_conditions(output: &DiagnoseMigrationHistoryOutput) -> Option<String> {
    let mut reset_reasons = Vec::new();

    for failed_migration_name in &output.failed_migration_names {
        reset_reasons.push(format!("The migration `{}` failed.", failed_migration_name));
    }

    for edited_migration_name in &output.edited_migration_names {
        reset_reasons.push(format!(
            "The migration `{}` was modified after it was applied.",
            edited_migration_name
        ))
    }

    if let Some(DriftDiagnostic::DriftDetected { rollback }) = &output.drift {
        tracing::info!(rollback = rollback.as_str(), "DriftDetected diagnostic");

        reset_reasons
            .push("Drift detected: Your database schema is not in sync with your migration history.".to_owned())
    }

    match &output.history {
        Some(HistoryDiagnostic::HistoriesDiverge { last_common_migration_name, unapplied_migration_names: _, unpersisted_migration_names }) => {
            let details = last_common_migration_name.as_ref().map(|last_common_migration_name|{
                format!(" Last common migration: `{}`. Migrations applied to the database but absent from the migrations directory are: {}", last_common_migration_name, unpersisted_migration_names.join(", "))
            }).unwrap_or_else(String::new);

            reset_reasons.push(format!("The migrations recorded in the database diverge from the local migrations directory.{}", details))
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

/// A suggested action for the CLI `migrate dev` command.
#[derive(Debug, Serialize)]
#[serde(tag = "tag", rename_all = "camelCase")]
pub enum DevAction {
    /// Reset the database.
    Reset {
        /// Why do we need to reset?
        reason: String,
    },
    /// Proceed to the next step.
    CreateMigration,
}

impl DevAction {
    /// Attempts to convert to a `Reset` and returns the reason.
    pub fn as_reset(&self) -> Option<&str> {
        match self {
            DevAction::Reset { reason } => Some(reason),
            _ => None,
        }
    }

    /// Returns `true` if the action is CreateMigration.
    pub fn is_create_migration(&self) -> bool {
        matches!(self, DevAction::CreateMigration)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dev_action_serializes_as_expected() {
        let reset = serde_json::to_value(DevAction::Reset {
            reason: "Because I said so".to_owned(),
        })
        .unwrap();

        assert_eq!(reset, json!({ "tag": "reset", "reason": "Because I said so" }));

        let create_migration = serde_json::to_value(DevAction::CreateMigration).unwrap();

        assert_eq!(create_migration, json!({ "tag": "createMigration" }));
    }
}

//! The InferMigrationSteps RPC method.

use super::MigrationStepsResultOutput;
use crate::{commands::command::*, migration_engine::MigrationEngine, *};
use datamodel::ast::{parser::parse, SchemaAst};
use migration_connector::*;
use serde::Deserialize;
use tracing::debug;
use tracing_error::SpanTrace;

pub struct InferMigrationStepsCommand<'a> {
    input: &'a InferMigrationStepsInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for InferMigrationStepsCommand<'a> {
    type Input = InferMigrationStepsInput;
    type Output = MigrationStepsResultOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Sync + Send + 'static,
    {
        let cmd = InferMigrationStepsCommand { input };
        debug!(?cmd.input);

        let connector = engine.connector();
        let migration_persistence = connector.migration_persistence();
        let database_migration_inferrer = connector.database_migration_inferrer();

        let assume_to_be_applied = cmd.assume_to_be_applied();

        cmd.validate_assumed_migrations_are_not_applied(migration_persistence.as_ref())
            .await?;

        let last_migration = migration_persistence.last().await?;
        let current_datamodel_ast = if let Some(migration) = last_migration.as_ref() {
            migration
                .parse_schema_ast()
                .map_err(|(err, schema)| CommandError::InvalidPersistedDatamodel(err, schema))?
        } else {
            SchemaAst::empty()
        };
        let assumed_datamodel_ast = engine
            .datamodel_calculator()
            .infer(&current_datamodel_ast, assume_to_be_applied.as_slice())?;
        let assumed_datamodel =
            datamodel::lift_ast(&assumed_datamodel_ast).map_err(CommandError::ProducedBadDatamodel)?;

        let next_datamodel = parse_datamodel(&cmd.input.datamodel)?;
        let version_check_errors = connector.check_database_version_compatibility(&next_datamodel);

        let next_datamodel_ast = parse(&cmd.input.datamodel).map_err(|err| {
            CommandError::Input(anyhow::anyhow!("{}", err.to_pretty_string("", &cmd.input.datamodel)))
        })?;

        let model_migration_steps = engine
            .datamodel_migration_steps_inferrer()
            .infer(&assumed_datamodel_ast, &next_datamodel_ast);

        let database_migration = database_migration_inferrer
            .infer(&assumed_datamodel, &next_datamodel, &model_migration_steps)
            .await?;

        let DestructiveChangeDiagnostics {
            warnings,
            errors: _,
            unexecutable_migrations,
        } = connector
            .destructive_changes_checker()
            .check(&database_migration)
            .await?;

        let (returned_datamodel_steps, returned_database_migration) =
            if !cmd.input.is_watch_migration() && last_migration.map(|mig| mig.is_watch_migration()).unwrap_or(false) {
                // Transition out of watch mode
                let last_non_watch_applied_migration = migration_persistence.last_non_watch_applied_migration().await?;
                let last_non_watch_datamodel_ast = last_non_watch_applied_migration
                    .as_ref()
                    .map(|m| m.parse_schema_ast())
                    .unwrap_or_else(|| Ok(SchemaAst::empty()))
                    .map_err(|(err, schema)| CommandError::InvalidPersistedDatamodel(err, schema))?;
                let last_non_watch_datamodel = last_non_watch_applied_migration
                    .map(|m| m.parse_datamodel())
                    .unwrap_or_else(|| Ok(Datamodel::empty()))
                    .map_err(|(err, schema)| CommandError::InvalidPersistedDatamodel(err, schema))?;
                let datamodel_steps = engine
                    .datamodel_migration_steps_inferrer()
                    .infer(&last_non_watch_datamodel_ast, &next_datamodel_ast);

                // The database migration since the last non-watch migration, so we can render all the steps applied
                // in watch mode to the migrations folder.
                let full_database_migration = database_migration_inferrer
                    .infer_from_datamodels(&last_non_watch_datamodel, &next_datamodel, &datamodel_steps)
                    .await?;

                (datamodel_steps, full_database_migration)
            } else {
                (model_migration_steps, database_migration)
            };

        let database_steps = connector
            .database_migration_step_applier()
            .render_steps_pretty(&returned_database_migration)?;

        debug!(?returned_datamodel_steps);

        Ok(MigrationStepsResultOutput {
            datamodel: datamodel::render_datamodel_to_string(&next_datamodel).unwrap(),
            datamodel_steps: returned_datamodel_steps,
            database_steps: serde_json::Value::Array(database_steps),
            errors: version_check_errors,
            warnings,
            general_errors: vec![],
            unexecutable_migrations,
        })
    }
}

impl InferMigrationStepsCommand<'_> {
    fn assume_to_be_applied(&self) -> Vec<MigrationStep> {
        self.input
            .assume_to_be_applied
            .clone()
            .or_else(|| {
                self.input.assume_applied_migrations.as_ref().map(|migrations| {
                    migrations
                        .into_iter()
                        .flat_map(|migration| migration.datamodel_steps.clone().into_iter())
                        .collect()
                })
            })
            .unwrap_or_else(Vec::new)
    }

    async fn validate_assumed_migrations_are_not_applied(
        &self,
        migration_persistence: &dyn MigrationPersistence,
    ) -> CommandResult<()> {
        if let Some(migrations) = self.input.assume_applied_migrations.as_ref() {
            for migration in migrations {
                if migration_persistence
                    .migration_is_already_applied(&migration.migration_id)
                    .await?
                {
                    return Err(CommandError::ConnectorError(ConnectorError {
                        user_facing_error: None,
                        kind: ErrorKind::Generic(anyhow::anyhow!(
                            "Input is invalid. Migration {} is already applied.",
                            migration.migration_id
                        )),
                        context: SpanTrace::capture(),
                    }));
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InferMigrationStepsInput {
    pub migration_id: String,
    #[serde(alias = "dataModel")]
    pub datamodel: String,
    /// Migration steps from migrations that have been inferred but not applied yet.
    ///
    /// These steps must be provided and correct for migration inferrence to work.
    pub assume_to_be_applied: Option<Vec<MigrationStep>>,
    pub assume_applied_migrations: Option<Vec<AppliedMigration>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedMigration {
    pub migration_id: String,
    pub datamodel_steps: Vec<MigrationStep>,
}

impl IsWatchMigration for InferMigrationStepsInput {
    fn is_watch_migration(&self) -> bool {
        self.migration_id.starts_with("watch")
    }
}

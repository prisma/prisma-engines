use crate::*;
use crate::{sql_schema_calculator::SqlSchemaCalculator, sql_schema_differ::SqlSchemaDiffer};
use datamodel::*;
use migration_connector::steps::MigrationStep;
use migration_connector::*;
use sql_migration::SqlMigrationStep;
use sql_schema_describer::*;

#[async_trait::async_trait]
impl DatabaseMigrationInferrer<SqlMigration> for SqlMigrationConnector {
    async fn infer(
        &self,
        _previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let current_database_schema: SqlSchema = self.describe_schema().await?;
        let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info(), self.flavour());
        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.database_info(),
            self.flavour(),
        ))
    }

    /// Infer the database migration steps, skipping the schema describer and assuming an empty database.
    fn infer_from_empty(&self, next: &Datamodel) -> ConnectorResult<SqlMigration> {
        let current_database_schema = SqlSchema::empty();
        let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info(), self.flavour());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.database_info(),
            self.flavour(),
        ))
    }

    fn infer_from_datamodels(
        &self,
        previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let current_database_schema: SqlSchema =
            SqlSchemaCalculator::calculate(previous, self.database_info(), self.flavour());
        let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info(), self.flavour());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.database_info(),
            self.flavour(),
        ))
    }

    #[tracing::instrument(skip(self, previous_migrations, target_schema))]
    async fn infer_next_migration(
        &self,
        previous_migrations: &[MigrationDirectory],
        target_schema: &Datamodel,
    ) -> ConnectorResult<SqlMigration> {
        let current_database_schema = self
            .flavour()
            .sql_schema_from_migration_history(previous_migrations, self.conn())
            .await?;
        let expected_database_schema =
            SqlSchemaCalculator::calculate(target_schema, self.database_info(), self.flavour());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.database_info(),
            self.flavour(),
        ))
    }

    async fn detect_drift(&self, applied_migrations: &[MigrationDirectory]) -> ConnectorResult<bool> {
        let expected_schema = self
            .flavour()
            .sql_schema_from_migration_history(applied_migrations, self.conn())
            .await?;

        let actual_schema = self.describe_schema().await?;

        let diff =
            SqlSchemaDiffer::diff(&actual_schema, &expected_schema, self.flavour(), self.database_info()).into_steps();

        Ok(!diff.is_empty())
    }
}

fn infer(
    current_database_schema: SqlSchema,
    expected_database_schema: SqlSchema,
    database_info: &DatabaseInfo,
    flavour: &dyn SqlFlavour,
) -> SqlMigration {
    let steps = SqlSchemaDiffer::diff(
        &current_database_schema,
        &expected_database_schema,
        flavour,
        &database_info,
    )
    .into_steps();

    SqlMigration {
        before: current_database_schema,
        after: expected_database_schema,
        steps,
    }
}

pub(crate) fn wrap_as_step<T, F>(steps: Vec<T>, wrap_fn: F) -> impl Iterator<Item = SqlMigrationStep>
where
    F: Fn(T) -> SqlMigrationStep,
{
    steps.into_iter().map(wrap_fn)
}

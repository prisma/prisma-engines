use crate::{
    flavour::SqlFlavour,
    pair::Pair,
    sql_migration::{SqlMigration, SqlMigrationStep},
    sql_schema_calculator, sql_schema_differ, SqlMigrationConnector,
};
use datamodel::Datamodel;
use migration_connector::{
    steps::MigrationStep, ConnectorResult, DatabaseMigrationInferrer, MigrationConnector, MigrationDirectory,
};
use sql_schema_describer::SqlSchema;

#[async_trait::async_trait]
impl DatabaseMigrationInferrer<SqlMigration> for SqlMigrationConnector {
    async fn infer(
        &self,
        _previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let connection = self.conn().await?;
        let current_database_schema = self.describe_schema().await?;
        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(next, connection.flavour());
        Ok(infer(
            current_database_schema,
            expected_database_schema,
            connection.flavour(),
        ))
    }

    /// Infer the database migration steps, skipping the schema describer and assuming an empty database.
    fn infer_from_empty(&self, next: &Datamodel) -> ConnectorResult<SqlMigration> {
        let flavour = self.abstract_flavour().as_ref();
        let current_database_schema = SqlSchema::empty();
        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(next, flavour);

        Ok(infer(current_database_schema, expected_database_schema, flavour))
    }

    fn infer_from_datamodels(
        &self,
        previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let flavour = self.abstract_flavour().as_ref();
        let current_database_schema = sql_schema_calculator::calculate_sql_schema(previous, flavour);
        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(next, flavour);

        Ok(infer(current_database_schema, expected_database_schema, flavour))
    }

    #[tracing::instrument(skip(self, previous_migrations, target_schema))]
    async fn infer_next_migration(
        &self,
        previous_migrations: &[MigrationDirectory],
        target_schema: &Datamodel,
    ) -> ConnectorResult<SqlMigration> {
        let connection = self.conn().await?;
        let current_database_schema = connection
            .flavour()
            .sql_schema_from_migration_history(previous_migrations, &connection)
            .await?;

        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(target_schema, connection.flavour());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            connection.flavour(),
        ))
    }

    #[tracing::instrument(skip(self, applied_migrations))]
    async fn calculate_drift(&self, applied_migrations: &[MigrationDirectory]) -> ConnectorResult<Option<String>> {
        let connection = self.conn().await?;

        let expected_schema = connection
            .flavour()
            .sql_schema_from_migration_history(applied_migrations, &connection)
            .await?;

        let actual_schema = self.describe_schema().await?;

        let steps =
            sql_schema_differ::calculate_steps(Pair::new(&actual_schema, &expected_schema), connection.flavour());

        if steps.is_empty() {
            return Ok(None);
        }

        let migration = SqlMigration {
            before: actual_schema,
            after: expected_schema,
            steps: steps,
        };

        let diagnostics = self.destructive_change_checker().pure_check(&migration);

        let rollback = self
            .database_migration_step_applier()
            .render_script(&migration, &diagnostics);

        Ok(Some(rollback))
    }

    #[tracing::instrument(skip(self, migrations))]
    async fn validate_migrations(&self, migrations: &[MigrationDirectory]) -> ConnectorResult<()> {
        let conn = self.conn().await?;

        conn.flavour()
            .sql_schema_from_migration_history(migrations, &conn)
            .await?;

        Ok(())
    }
}

fn infer(
    current_database_schema: SqlSchema,
    expected_database_schema: SqlSchema,
    flavour: &dyn SqlFlavour,
) -> SqlMigration {
    let steps =
        sql_schema_differ::calculate_steps(Pair::new(&current_database_schema, &expected_database_schema), flavour);

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

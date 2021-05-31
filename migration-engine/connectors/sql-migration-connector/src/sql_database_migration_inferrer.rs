use crate::{
    flavour::SqlFlavour,
    pair::Pair,
    sql_migration::{SqlMigration, SqlMigrationStep},
    sql_schema_calculator, sql_schema_differ, SqlMigrationConnector,
};
use datamodel::{walkers::walk_models, Configuration, Datamodel};
use migration_connector::{ConnectorResult, DatabaseMigrationInferrer, MigrationConnector, MigrationDirectory};
use sql_schema_describer::{walkers::SqlSchemaExt, SqlSchema};

#[async_trait::async_trait]
impl DatabaseMigrationInferrer<SqlMigration> for SqlMigrationConnector {
    async fn infer(&self, next: (&Configuration, &Datamodel)) -> ConnectorResult<SqlMigration> {
        let current_database_schema: SqlSchema = self.describe_schema().await?;
        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(next, self.flavour());
        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.flavour(),
            next.1,
        ))
    }

    /// Infer the database migration steps, skipping the schema describer and assuming an empty database.
    fn infer_from_empty(&self, next: (&Configuration, &Datamodel)) -> ConnectorResult<SqlMigration> {
        let current_database_schema = SqlSchema::empty();
        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(next, self.flavour());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.flavour(),
            next.1,
        ))
    }

    #[tracing::instrument(skip(self, previous_migrations, target_schema))]
    async fn infer_next_migration(
        &self,
        previous_migrations: &[MigrationDirectory],
        target_schema: (&Configuration, &Datamodel),
    ) -> ConnectorResult<SqlMigration> {
        let current_database_schema = self
            .flavour()
            .sql_schema_from_migration_history(previous_migrations, self.conn(), self)
            .await?;
        let expected_database_schema = sql_schema_calculator::calculate_sql_schema(target_schema, self.flavour());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.flavour(),
            target_schema.1,
        ))
    }

    #[tracing::instrument(skip(self, applied_migrations))]
    async fn calculate_drift(&self, applied_migrations: &[MigrationDirectory]) -> ConnectorResult<Option<String>> {
        let expected_schema = self
            .flavour()
            .sql_schema_from_migration_history(applied_migrations, self.conn(), self)
            .await?;

        let actual_schema = self.describe_schema().await?;

        let steps = sql_schema_differ::calculate_steps(Pair::new(&actual_schema, &expected_schema), self.flavour());

        if steps.is_empty() {
            return Ok(None);
        }

        let migration = SqlMigration {
            before: actual_schema,
            after: expected_schema,
            added_columns_with_virtual_defaults: Vec::new(),
            steps,
        };

        let diagnostics = self.destructive_change_checker().pure_check(&migration);

        let rollback = self
            .database_migration_step_applier()
            .render_script(&migration, &diagnostics);

        Ok(Some(rollback))
    }

    #[tracing::instrument(skip(self, migrations))]
    async fn validate_migrations(&self, migrations: &[MigrationDirectory]) -> ConnectorResult<()> {
        self.flavour()
            .sql_schema_from_migration_history(migrations, self.conn(), self)
            .await?;

        Ok(())
    }
}

fn infer(
    current_database_schema: SqlSchema,
    expected_database_schema: SqlSchema,
    flavour: &dyn SqlFlavour,
    next_datamodel: &Datamodel,
) -> SqlMigration {
    let steps =
        sql_schema_differ::calculate_steps(Pair::new(&current_database_schema, &expected_database_schema), flavour);
    let next_schema = &expected_database_schema;

    let added_columns_with_virtual_defaults: Vec<(usize, usize)> = walk_added_columns(&steps)
        .map(|(table_index, column_index)| {
            let table = next_schema.table_walker_at(table_index);
            let column = table.column_at(column_index);

            (table, column)
        })
        .filter(|(table, column)| {
            walk_models(next_datamodel)
                .find(|model| model.database_name() == table.name())
                .and_then(|model| model.find_scalar_field(column.name()))
                .filter(|field| {
                    field
                        .default_value()
                        .map(|default| default.is_uuid() || default.is_cuid())
                        .unwrap_or(false)
                })
                .is_some()
        })
        .map(move |(table, column)| (table.table_index(), column.column_index()))
        .collect();

    SqlMigration {
        added_columns_with_virtual_defaults,
        before: current_database_schema,
        after: expected_database_schema,
        steps,
    }
}

/// List all the columns added in the migration, either by alter table steps or
/// redefine table steps.
///
/// The return value should be interpreted as an iterator over `(table_index,
/// column_index)` in the `next` schema.
fn walk_added_columns(steps: &[SqlMigrationStep]) -> impl Iterator<Item = (usize, usize)> + '_ {
    steps
        .iter()
        .filter_map(|step| step.as_alter_table())
        .flat_map(move |alter_table| {
            alter_table
                .changes
                .iter()
                .filter_map(|change| change.as_add_column())
                .map(move |column| -> (usize, usize) { (*alter_table.table_index.next(), column.column_index) })
        })
        .chain(
            steps
                .iter()
                .filter_map(|step| step.as_redefine_tables())
                .flat_map(|redefine_tables| redefine_tables)
                .flat_map(move |table| {
                    table
                        .added_columns
                        .iter()
                        .map(move |column_index| (*table.table_index.next(), *column_index))
                }),
        )
}

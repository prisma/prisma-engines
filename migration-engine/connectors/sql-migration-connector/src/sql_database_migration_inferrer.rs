// mod sqlite;

use crate::sql_schema_calculator::SqlSchemaCalculator;
use crate::sql_schema_differ::{SqlSchemaDiff, SqlSchemaDiffer};
use crate::*;
use datamodel::*;
use migration_connector::steps::MigrationStep;
use migration_connector::*;
use sql_schema_describer::*;

pub struct SqlDatabaseMigrationInferrer<'a> {
    pub connector: &'a crate::SqlMigrationConnector,
}

impl Component for SqlDatabaseMigrationInferrer<'_> {
    fn connector(&self) -> &crate::SqlMigrationConnector {
        self.connector
    }
}

#[async_trait::async_trait]
impl DatabaseMigrationInferrer<SqlMigration> for SqlDatabaseMigrationInferrer<'_> {
    async fn infer(
        &self,
        _previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let fut = async {
            let current_database_schema: SqlSchema = self.describe().await?;
            let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info());
            infer(
                &current_database_schema,
                &expected_database_schema,
                self.database_info(),
                self.flavour(),
            )
        };

        catch(&self.connection_info(), fut).await
    }

    async fn infer_from_datamodels(
        &self,
        previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let result: SqlResult<SqlMigration> = (|| {
            let current_database_schema: SqlSchema = SqlSchemaCalculator::calculate(previous, self.database_info());
            let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info());
            infer(
                &current_database_schema,
                &expected_database_schema,
                self.database_info(),
                self.flavour(),
            )
        })();

        result.map_err(|sql_error| sql_error.into_connector_error(self.connection_info()))
    }
}

fn infer(
    current_database_schema: &SqlSchema,
    expected_database_schema: &SqlSchema,
    database_info: &DatabaseInfo,
    flavour: &dyn SqlFlavour,
) -> SqlResult<SqlMigration> {
    let (original_steps, corrected_steps) = infer_database_migration_steps_and_fix(
        &current_database_schema,
        &expected_database_schema,
        database_info,
        flavour,
    )?;
    let (_, rollback) = infer_database_migration_steps_and_fix(
        &expected_database_schema,
        &current_database_schema,
        database_info,
        flavour,
    )?;
    Ok(SqlMigration {
        before: current_database_schema.clone(),
        after: expected_database_schema.clone(),
        original_steps,
        corrected_steps,
        rollback,
    })
}

fn infer_database_migration_steps_and_fix(
    from: &SqlSchema,
    to: &SqlSchema,
    database_info: &DatabaseInfo,
    flavour: &dyn SqlFlavour,
) -> SqlResult<(Vec<SqlMigrationStep>, Vec<SqlMigrationStep>)> {
    let diff: SqlSchemaDiff = SqlSchemaDiffer::diff(&from, &to, flavour, &database_info);

    // let corrected_steps = if sql_family.is_sqlite() {
    //     sqlite::fix(diff, &from, &to, &schema_name, database_info, flavour)?
    // } else {
    //     diff.into_steps()
    // };

    Ok((
        SqlSchemaDiffer::diff(&from, &to, flavour, &database_info).into_steps(),
        diff.into_steps(),
    ))
}

pub fn wrap_as_step<T, F>(steps: Vec<T>, wrap_fn: F) -> impl Iterator<Item = SqlMigrationStep>
where
    F: Fn(T) -> SqlMigrationStep,
{
    steps.into_iter().map(wrap_fn)
}

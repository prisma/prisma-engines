use crate::*;
use crate::{sql_schema_calculator::SqlSchemaCalculator, sql_schema_differ::SqlSchemaDiffer};
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
            Ok(infer(
                current_database_schema,
                expected_database_schema,
                self.database_info(),
                self.flavour(),
            ))
        };

        catch(&self.connection_info(), fut).await
    }

    fn infer_from_datamodels(
        &self,
        previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let current_database_schema: SqlSchema = SqlSchemaCalculator::calculate(previous, self.database_info());
        let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info());

        Ok(infer(
            current_database_schema,
            expected_database_schema,
            self.database_info(),
            self.flavour(),
        ))
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

pub fn wrap_as_step<T, F>(steps: Vec<T>, wrap_fn: F) -> impl Iterator<Item = SqlMigrationStep>
where
    F: Fn(T) -> SqlMigrationStep,
{
    steps.into_iter().map(wrap_fn)
}

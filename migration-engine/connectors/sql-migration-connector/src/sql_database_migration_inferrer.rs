mod sqlite;

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
            let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info())?;
            infer(
                &current_database_schema,
                &expected_database_schema,
                self.schema_name(),
                self.sql_family(),
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
            let current_database_schema: SqlSchema = SqlSchemaCalculator::calculate(previous, self.database_info())?;
            let expected_database_schema = SqlSchemaCalculator::calculate(next, self.database_info())?;
            infer(
                &current_database_schema,
                &expected_database_schema,
                self.schema_name(),
                self.sql_family(),
            )
        })();

        result.map_err(|sql_error| sql_error.into_connector_error(self.connection_info()))
    }
}

fn infer(
    current_database_schema: &SqlSchema,
    expected_database_schema: &SqlSchema,
    schema_name: &str,
    sql_family: SqlFamily,
) -> SqlResult<SqlMigration> {
    let (original_steps, corrected_steps) = infer_database_migration_steps_and_fix(
        &current_database_schema,
        &expected_database_schema,
        &schema_name,
        sql_family,
    )?;
    let (_, rollback) = infer_database_migration_steps_and_fix(
        &expected_database_schema,
        &current_database_schema,
        &schema_name,
        sql_family,
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
    schema_name: &str,
    sql_family: SqlFamily,
) -> SqlResult<(Vec<SqlMigrationStep>, Vec<SqlMigrationStep>)> {
    let diff: SqlSchemaDiff = SqlSchemaDiffer::diff(&from, &to, sql_family);

    let corrected_steps = if sql_family.is_sqlite() {
        sqlite::fix(diff, &from, &to, &schema_name)?
    } else {
        fix_id_column_type_change(&from, &to, schema_name, diff.into_steps(), sql_family)?
    };

    Ok((
        SqlSchemaDiffer::diff(&from, &to, sql_family).into_steps(),
        corrected_steps,
    ))
}

fn fix_id_column_type_change(
    from: &SqlSchema,
    to: &SqlSchema,
    _schema_name: &str,
    steps: Vec<SqlMigrationStep>,
    sql_family: SqlFamily,
) -> SqlResult<Vec<SqlMigrationStep>> {
    let has_id_type_change = steps
        .iter()
        .find(|step| match step {
            SqlMigrationStep::AlterTable(alter_table) => {
                if let Ok(current_table) = from.table(&alter_table.table.name) {
                    let change_to_id_column = alter_table.changes.iter().find(|c| match c {
                        TableChange::AlterColumn(alter_column) => {
                            let current_column = current_table.column_bang(&alter_column.name);
                            let current_column_type = &current_column.tpe;
                            let has_type_changed = current_column_type.family != alter_column.column.tpe.family; // TODO: take into account raw type
                            let is_part_of_pk = current_table
                                .primary_key
                                .clone()
                                .map(|pk| pk.columns)
                                .unwrap_or(vec![])
                                .contains(&alter_column.name);
                            is_part_of_pk && has_type_changed
                        }
                        _ => false,
                    });
                    change_to_id_column.is_some()
                } else {
                    false
                }
            }
            _ => false,
        })
        .is_some();

    // TODO: There's probably a much more graceful way to handle this. But this would also involve a lot of data loss probably. Let's tackle that after P Day
    if has_id_type_change {
        let tables_to_drop: Vec<String> = from
            .tables
            .iter()
            .filter(|t| t.name != MIGRATION_TABLE_NAME)
            .map(|t| t.name.clone())
            .collect();
        let mut radical_steps = Vec::with_capacity(tables_to_drop.len());
        radical_steps.extend(
            tables_to_drop
                .into_iter()
                .map(|name| DropTable { name })
                .map(SqlMigrationStep::DropTable),
        );
        let diff_from_empty: SqlSchemaDiff = SqlSchemaDiffer::diff(&SqlSchema::empty(), &to, sql_family);
        let mut steps_from_empty = diff_from_empty.into_steps();
        radical_steps.append(&mut steps_from_empty);

        Ok(radical_steps)
    } else {
        Ok(steps)
    }
}

pub fn wrap_as_step<T, F>(steps: Vec<T>, mut wrap_fn: F) -> impl Iterator<Item = SqlMigrationStep>
where
    F: FnMut(T) -> SqlMigrationStep,
{
    steps.into_iter().map(move |x| wrap_fn(x))
}

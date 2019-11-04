use crate::{DropColumn, DropTable, DropTables, SqlError, SqlMigration, SqlMigrationStep, SqlResult, TableChange};
use migration_connector::*;
use quaint::ast::*;
use sql_connection::SyncSqlConnection;
use std::sync::Arc;

pub struct SqlDestructiveChangesChecker {
    pub schema_name: String,
    pub database: Arc<dyn SyncSqlConnection + Send + Sync>,
}

impl SqlDestructiveChangesChecker {
    fn check_table_drop(&self, table_name: &str, diagnostics: &mut DestructiveChangeDiagnostics) -> SqlResult<()> {
        let query = Select::from_table((self.schema_name.as_str(), table_name)).value(count(asterisk()));
        let result_set = self.database.query(query.into())?;
        let first_row = result_set.first().ok_or_else(|| {
            SqlError::Generic("No row was returned when checking for existing rows in dropped table.".to_owned())
        })?;
        let rows_count: i64 = first_row.at(0).and_then(|value| value.as_i64()).ok_or_else(|| {
            SqlError::Generic("No count was returned when checking for existing rows in dropped table.".to_owned())
        })?;

        if rows_count > 0 {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                    "You are about to drop the table `{table_name}`, which is not empty ({rows_count} rows).",
                    table_name = table_name,
                    rows_count = rows_count
                ),
            });
        }

        Ok(())
    }

    /// Emit a warning when we drop a column that contains non-null values.
    fn check_column_drop(
        &self,
        drop_column: &DropColumn,
        table: &sql_schema_describer::Table,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let query = Select::from_table((self.schema_name.as_str(), table.name.as_str()))
            .value(count(quaint::ast::Column::new(drop_column.name.as_str())))
            .so_that(drop_column.name.as_str().is_not_null());

        let values_count: i64 = self
            .database
            .query(query.into())
            .map_err(SqlError::from)
            .and_then(|result_set| {
                result_set
                    .first()
                    .as_ref()
                    .and_then(|row| row.at(0))
                    .and_then(|count| count.as_i64())
                    .ok_or_else(|| {
                        SqlError::Generic("Unexpected result set shape when checking dropped columns.".to_owned())
                    })
            })?;

        if values_count > 0 {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                    "You are about to drop the column `{column_name}` on the `{table_name}` table, which still contains {values_count} non-null values.",
                    column_name=drop_column.name,
                    table_name=&table.name,
                    values_count=values_count,
                )
            })
        }

        Ok(())
    }
}

impl DestructiveChangesChecker<SqlMigration> for SqlDestructiveChangesChecker {
    fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for step in &database_migration.original_steps {
            match step {
                SqlMigrationStep::AlterTable(alter_table) => {
                    alter_table
                        .changes
                        .iter()
                        .map(|change| match *change {
                            TableChange::DropColumn(ref drop_column) => {
                                // The table in alter_table is the updated table, but we want to
                                // check against the current state of the table.
                                //
                                // TODO: discuss whether Generic is the right error variant (should
                                // we have an InvariantViolation variant or similar?)
                                let before_table = database_migration
                                    .before
                                    .get_table(&alter_table.table.name)
                                    .ok_or_else(|| {
                                        SqlError::Generic(format!(
                                            "Internal Error: dropping column {} on previously-unknown table {}",
                                            drop_column.name, &alter_table.table.name
                                        ))
                                    })?;
                                self.check_column_drop(drop_column, before_table, &mut diagnostics)
                            }
                            _ => Ok(()),
                        })
                        .collect::<Result<(), SqlError>>()?;
                }
                // Here, check for each table we are going to delete if it is empty. If
                // not, return a warning.
                SqlMigrationStep::DropTable(DropTable { name }) => {
                    self.check_table_drop(name, &mut diagnostics)?;
                }
                SqlMigrationStep::DropTables(DropTables { names }) => {
                    for name in names {
                        self.check_table_drop(name, &mut diagnostics)?;
                    }
                }
                // do nothing
                _ => (),
            }
        }

        Ok(diagnostics)
    }
}

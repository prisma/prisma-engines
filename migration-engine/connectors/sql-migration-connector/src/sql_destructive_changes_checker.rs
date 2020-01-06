use crate::{
    AlterColumn, ConnectionInfo, DropColumn, DropTable, DropTables, SqlError, SqlMigration, SqlMigrationStep,
    SqlResult, TableChange,
};
use migration_connector::*;
use quaint::{ast::*, prelude::Queryable};
use sql_schema_describer::SqlSchemaDescriberBackend;
use std::sync::Arc;

pub struct SqlDestructiveChangesChecker {
    pub connection_info: ConnectionInfo,
    pub schema_name: String,
    pub database: Arc<dyn Queryable + Send + Sync>,
    pub database_describer: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static>,
}

impl SqlDestructiveChangesChecker {
    async fn check_table_drop(
        &self,
        table_name: &str,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let query = Select::from_table((self.schema_name.as_str(), table_name)).value(count(asterisk()));
        let result_set = self.database.query(query.into()).await?;
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

    async fn count_values_in_column(&self, column_name: &str, table: &sql_schema_describer::Table) -> SqlResult<i64> {
        let query = Select::from_table((self.schema_name.as_str(), table.name.as_str()))
            .value(count(quaint::ast::Column::new(column_name)))
            .so_that(column_name.is_not_null());

        let values_count: i64 = self
            .database
            .query(query.into())
            .await
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

        Ok(values_count)
    }

    /// Emit a warning when we drop a column that contains non-null values.
    async fn check_column_drop(
        &self,
        drop_column: &DropColumn,
        table: &sql_schema_describer::Table,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let values_count = self.count_values_in_column(&drop_column.name, table).await?;

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

    /// Emit a warning when we alter a column that contains non-null values. We will implement
    /// non-destructive alter column for a subset of changes in the future, but at the moment all
    /// alter columns are destructive.
    async fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        previous_table: &sql_schema_describer::Table,
        diagnostics: &mut DestructiveChangeDiagnostics,
    ) -> SqlResult<()> {
        let values_count = self.count_values_in_column(&alter_column.name, previous_table).await?;
        let previous_column = previous_table
            .column(&alter_column.name)
            .expect("unsupported column renaming");

        if values_count > 0 {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                                 "You are about to alter the column `{column_name}` on the `{table_name}` table, which still contains {values_count} non-null values. The data in that column will be lost.",
                                 column_name=alter_column.name,
                                 table_name=&previous_table.name,
                                 values_count=values_count,
                             )
            })
        } else if previous_table.is_part_of_foreign_key(&alter_column.column.name)
            && alter_column.column.default.is_none()
            && previous_column.default.is_some()
        {
            diagnostics.add_warning(MigrationWarning {
                description: format!(
                    "The migration is about to remove a default value on the foreign key field `{}.{}`.",
                    previous_table.name, alter_column.name,
                ),
            })
        }

        Ok(())
    }

    async fn check_impl(&self, database_migration: &SqlMigration) -> SqlResult<DestructiveChangeDiagnostics> {
        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for step in &database_migration.original_steps {
            match step {
                SqlMigrationStep::AlterTable(alter_table) => {
                    // The table in alter_table is the updated table, but we want to
                    // check against the current state of the table.
                    let before_table = database_migration
                        .before
                        .get_table(&alter_table.table.name);

                    if let Some(before_table) = before_table {
                        for change in &alter_table.changes {
                            match *change {
                                TableChange::DropColumn(ref drop_column) => {
                                    self.check_column_drop(drop_column, before_table, &mut diagnostics)
                                        .await?
                                }
                                TableChange::AlterColumn(ref alter_column) => {
                                    self.check_alter_column(alter_column, before_table, &mut diagnostics)
                                        .await?
                                }
                                _ => (),
                            }
                        }
                    }

                }
                // Here, check for each table we are going to delete if it is empty. If
                // not, return a warning.
                SqlMigrationStep::DropTable(DropTable { name }) => {
                    self.check_table_drop(name, &mut diagnostics).await?;
                }
                SqlMigrationStep::DropTables(DropTables { names }) => {
                    for name in names {
                        self.check_table_drop(name, &mut diagnostics).await?;
                    }
                }
                // do nothing
                _ => (),
            }
        }

        Ok(diagnostics)
    }
}

#[async_trait::async_trait]
impl DestructiveChangesChecker<SqlMigration> for SqlDestructiveChangesChecker {
    async fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(database_migration)
            .await
            .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info))
    }
}

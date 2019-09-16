use crate::{DropTable, DropTables, MigrationDatabase, SqlError, SqlMigration, SqlMigrationStep, SqlResult};
use migration_connector::*;
use prisma_query::ast::*;
use std::sync::Arc;

pub struct SqlDestructiveChangesChecker {
    pub schema_name: String,
    pub database: Arc<dyn MigrationDatabase + Send + Sync>,
}

impl SqlDestructiveChangesChecker {
    fn check_table_drop(&self, table_name: &str) -> SqlResult<Option<MigrationWarning>> {
        let query = Select::from_table((self.schema_name.as_str(), table_name)).value(count(asterisk()));
        let result_set = self.database.query(&self.schema_name, query.into())?;
        let first_row = result_set.first().ok_or_else(|| {
            SqlError::Generic("No row was returned when checking for existing rows in dropped table.".to_owned())
        })?;
        let rows_count: i64 = first_row.at(0).and_then(|value| value.as_i64()).ok_or_else(|| {
            SqlError::Generic("No count was returned when checking for existing rows in dropped table.".to_owned())
        })?;

        if rows_count > 0 {
            Ok(Some(MigrationWarning {
                description: format!(
                    "You are about to drop the table `{table_name}`, which is not empty ({rows_count} rows).",
                    table_name = table_name,
                    rows_count = rows_count
                ),
            }))
        } else {
            Ok(None)
        }
    }
}

#[allow(unused, dead_code)]
impl DestructiveChangesChecker<SqlMigration> for SqlDestructiveChangesChecker {
    fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for step in &database_migration.steps {
            // Here, check for each table we are going to delete if it is empty. If not, return a
            // warning.
            match step {
                SqlMigrationStep::DropTable(DropTable { name }) => {
                    diagnostics.add_warning(self.check_table_drop(name)?);
                }
                SqlMigrationStep::DropTables(DropTables { names }) => {
                    for name in names {
                        diagnostics.add_warning(self.check_table_drop(name)?);
                    }
                }
                // do nothing
                _ => (),
            }
        }

        Ok(diagnostics)
    }
}

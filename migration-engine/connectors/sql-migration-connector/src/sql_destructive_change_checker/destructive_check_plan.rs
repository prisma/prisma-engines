use super::{
    check::Check, database_inspection_results::DatabaseInspectionResults,
    unexecutable_step_check::UnexecutableStepCheck, warning_check::SqlMigrationWarningCheck,
};
use crate::{SqlError, SqlResult};
use migration_connector::{DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration};
use quaint::prelude::Queryable;
use std::time::Duration;
use tokio::time::{timeout, Elapsed};

const DESTRUCTIVE_TIMEOUT_DURATION: Duration = Duration::from_secs(60);

/// A DestructiveCheckPlan is the collection of destructive change checks
/// ([Check](trait.Check.html)) for a given migration. It has an `execute` method that performs
/// database inspection and renders user-facing messages based on the checks.
#[derive(Debug)]
pub(crate) struct DestructiveCheckPlan {
    warnings: Vec<(SqlMigrationWarningCheck, usize)>,
    unexecutable_migrations: Vec<(UnexecutableStepCheck, usize)>,
}

impl DestructiveCheckPlan {
    pub(super) fn new() -> Self {
        DestructiveCheckPlan {
            warnings: Vec::new(),
            unexecutable_migrations: Vec::new(),
        }
    }

    pub(super) fn push_warning(&mut self, warning: SqlMigrationWarningCheck, step_index: usize) {
        self.warnings.push((warning, step_index))
    }

    pub(super) fn push_unexecutable(&mut self, unexecutable_migration: UnexecutableStepCheck, step_index: usize) {
        self.unexecutable_migrations.push((unexecutable_migration, step_index))
    }

    /// Inspect the current database state to qualify and render destructive change warnings and
    /// errors.
    ///
    /// For example, dropping a table that has 0 rows can be considered safe.
    #[tracing::instrument(skip(conn, schema_name), level = "debug")]
    pub(super) async fn execute(
        &self,
        schema_name: &str,
        conn: &dyn Queryable,
    ) -> SqlResult<DestructiveChangeDiagnostics> {
        let mut results = DatabaseInspectionResults::default();

        let inspection = async {
            for (unexecutable, _idx) in &self.unexecutable_migrations {
                self.inspect_for_check(unexecutable, &mut results, schema_name, conn)
                    .await?;
            }

            for (warning, _idx) in &self.warnings {
                self.inspect_for_check(warning, &mut results, schema_name, conn).await?;
            }

            Ok::<(), SqlError>(())
        };

        // Ignore the timeout error, we will still return useful warnings.
        match timeout(DESTRUCTIVE_TIMEOUT_DURATION, inspection).await {
            Ok(Ok(())) | Err(Elapsed { .. }) => (),
            Ok(Err(err)) => return Err(err),
        };

        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for (unexecutable, step_index) in &self.unexecutable_migrations {
            if let Some(message) = unexecutable.evaluate(&results) {
                diagnostics.unexecutable_migrations.push(UnexecutableMigration {
                    description: message,
                    step_index: *step_index,
                })
            }
        }

        for (warning, step_index) in &self.warnings {
            if let Some(message) = warning.evaluate(&results) {
                diagnostics.warnings.push(MigrationWarning {
                    description: message,
                    step_index: *step_index,
                })
            }
        }

        Ok(diagnostics)
    }

    /// Perform the database inspection for a given [`Check`](trait.Check.html).
    pub(super) async fn inspect_for_check(
        &self,
        check: &(dyn Check + Send + Sync + 'static),
        results: &mut DatabaseInspectionResults,
        schema_name: &str,
        conn: &dyn Queryable,
    ) -> SqlResult<()> {
        if let Some(table) = check.needed_table_row_count() {
            if results.get_row_count(table).is_none() {
                let count = count_rows_in_table(table, schema_name, conn).await?;
                results.set_row_count(table.to_owned(), count)
            }
        }

        if let Some((table, column)) = check.needed_column_value_count() {
            if let (_, None) = results.get_row_and_non_null_value_count(table, column) {
                let count = count_values_in_column(column, table, schema_name, conn).await?;
                results.set_value_count(table.to_owned().into(), column.to_owned().into(), count);
            }
        }

        Ok(())
    }

    /// Return hypothetical warnings and errors, without performing any database
    /// IO. This is useful when we want to return diagnostics in reference to a
    /// database we cannot check directly. For example when we want to emit
    /// warnings about the production database, when creating a migration in
    /// development.
    pub(super) fn pure_check(&self) -> DestructiveChangeDiagnostics {
        let results = DatabaseInspectionResults::default();
        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for (unexecutable, step_index) in &self.unexecutable_migrations {
            if let Some(message) = unexecutable.evaluate(&results) {
                diagnostics.unexecutable_migrations.push(UnexecutableMigration {
                    description: message,
                    step_index: *step_index,
                })
            }
        }

        for (warning, step_index) in &self.warnings {
            if let Some(message) = warning.evaluate(&results) {
                diagnostics.warnings.push(MigrationWarning {
                    description: message,
                    step_index: *step_index,
                })
            }
        }

        diagnostics
    }
}

async fn count_rows_in_table(table_name: &str, schema_name: &str, conn: &dyn Queryable) -> SqlResult<i64> {
    use quaint::ast::*;

    let query = Select::from_table((schema_name, table_name)).value(count(asterisk()));
    let result_set = conn.query(query.into()).await?;
    let rows_count = result_set
        .first()
        .ok_or_else(|| {
            SqlError::Generic(anyhow::anyhow!(
                "No row was returned when checking for existing rows in the `{}` table.",
                table_name
            ))
        })?
        .at(0)
        .and_then(|value| value.as_i64())
        .ok_or_else(|| {
            SqlError::Generic(anyhow::anyhow!(
                "No count was returned when checking for existing rows in the `{}` table.",
                table_name
            ))
        })?;

    Ok(rows_count)
}

async fn count_values_in_column(
    column_name: &str,
    table: &str,
    schema_name: &str,
    conn: &dyn Queryable,
) -> SqlResult<i64> {
    use quaint::ast::*;

    let query = Select::from_table((schema_name, table))
        .value(count(quaint::ast::Column::new(column_name)))
        .so_that(column_name.is_not_null());

    let values_count: i64 = conn
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
                    SqlError::Generic(anyhow::anyhow!(
                        "Unexpected result set shape when checking dropped columns."
                    ))
                })
        })?;

    Ok(values_count)
}

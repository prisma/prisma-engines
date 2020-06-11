use super::{
    check::Check, database_inspection_results::DatabaseInspectionResults,
    unexecutable_step_check::UnexecutableStepCheck, warning_check::SqlMigrationWarning,
};
use crate::{SqlError, SqlResult};
use migration_connector::{DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration};
use quaint::prelude::Queryable;

/// A DestructiveCheckPlan is the collection of destructive change checks
/// ([Check](trait.Check.html)) for a given migration. It has an `execute` method that performs
/// database inspection and renders user-facing messages based on the checks.
#[derive(Debug)]
pub(super) struct DestructiveCheckPlan {
    warnings: Vec<SqlMigrationWarning>,
    unexecutable_migrations: Vec<UnexecutableStepCheck>,
}

impl DestructiveCheckPlan {
    pub(super) fn new() -> Self {
        DestructiveCheckPlan {
            warnings: Vec::new(),
            unexecutable_migrations: Vec::new(),
        }
    }

    pub(super) fn push_warning(&mut self, warning: SqlMigrationWarning) {
        self.warnings.push(warning)
    }

    pub(super) fn push_unexecutable(&mut self, unexecutable_migration: UnexecutableStepCheck) {
        self.unexecutable_migrations.push(unexecutable_migration)
    }

    /// Inspect the current database state to qualify and render destructive change warnings and
    /// errors.
    ///
    /// For example, dropping a table that has 0 rows can be considered safe.
    #[tracing::instrument(skip(conn, schema_name), level = "debug")]
    pub(super) async fn execute(
        &mut self,
        schema_name: &str,
        conn: &dyn Queryable,
    ) -> SqlResult<DestructiveChangeDiagnostics> {
        let mut results = DatabaseInspectionResults::default();

        for unexecutable in &self.unexecutable_migrations {
            self.inspect_for_check(unexecutable, &mut results, schema_name, conn)
                .await?;
        }

        for warning in &self.warnings {
            self.inspect_for_check(warning, &mut results, schema_name, conn).await?;
        }

        let mut diagnostics = DestructiveChangeDiagnostics::new();

        for unexecutable in &self.unexecutable_migrations {
            if let Some(message) = unexecutable.evaluate(&results) {
                diagnostics
                    .unexecutable_migrations
                    .push(UnexecutableMigration { description: message })
            }
        }

        for warning in &self.warnings {
            if let Some(message) = warning.evaluate(&results) {
                diagnostics.warnings.push(MigrationWarning { description: message })
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

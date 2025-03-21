use super::{
    check::Check, database_inspection_results::DatabaseInspectionResults,
    unexecutable_step_check::UnexecutableStepCheck, warning_check::SqlMigrationWarningCheck,
};
use crate::flavour::SqlConnector;
use schema_connector::{
    ConnectorError, ConnectorResult, DestructiveChangeDiagnostics, MigrationWarning, UnexecutableMigration,
};
use std::time::Duration;
use tokio::time::{error::Elapsed, timeout};

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
    pub fn new() -> Self {
        DestructiveCheckPlan {
            warnings: Vec::new(),
            unexecutable_migrations: Vec::new(),
        }
    }

    pub fn push_warning(&mut self, warning: SqlMigrationWarningCheck, step_index: usize) {
        self.warnings.push((warning, step_index))
    }

    pub fn push_unexecutable(&mut self, unexecutable_migration: UnexecutableStepCheck, step_index: usize) {
        self.unexecutable_migrations.push((unexecutable_migration, step_index))
    }

    /// Inspect the current database state to qualify and render destructive change warnings and
    /// errors.
    ///
    /// For example, dropping a table that has 0 rows can be considered safe.
    #[tracing::instrument(skip(connector), level = "debug")]
    pub async fn execute(
        &self,
        connector: &mut (dyn SqlConnector + Send + Sync),
    ) -> ConnectorResult<DestructiveChangeDiagnostics> {
        let mut results = DatabaseInspectionResults::default();

        let inspection = async {
            for (unexecutable, _idx) in &self.unexecutable_migrations {
                self.inspect_for_check(unexecutable, connector, &mut results).await?;
            }

            for (warning, _idx) in &self.warnings {
                self.inspect_for_check(warning, connector, &mut results).await?;
            }

            Ok::<(), ConnectorError>(())
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
    pub async fn inspect_for_check(
        &self,
        check: &(dyn Check + Send + Sync + 'static),
        connector: &mut (dyn SqlConnector + Send + Sync),
        results: &mut DatabaseInspectionResults,
    ) -> ConnectorResult<()> {
        let mut checker = connector.dialect().destructive_change_checker();
        if let Some(table) = check.needed_table_row_count() {
            if results.get_row_count(&table).is_none() {
                let count = checker.count_rows_in_table(connector, &table).await?;
                results.set_row_count(table.to_owned(), count)
            }
        }

        if let Some(column) = check.needed_column_value_count() {
            if let (_, None) = results.get_row_and_non_null_value_count(&column) {
                let count = checker.count_values_in_column(connector, &column).await?;
                results.set_value_count(column, count);
            }
        }

        Ok(())
    }

    /// Return hypothetical warnings and errors, without performing any database
    /// IO. This is useful when we want to return diagnostics in reference to a
    /// database we cannot check directly. For example when we want to emit
    /// warnings about the production database, when creating a migration in
    /// development.
    pub fn pure_check(&self) -> DestructiveChangeDiagnostics {
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

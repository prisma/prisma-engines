mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use super::DestructiveCheckPlan;
use crate::{sql_migration::AlterColumn, sql_schema_differ::ColumnChanges, sql_schema_differ::ColumnDiffer};

/// Flavour-specific destructive change checks.
pub(crate) trait DestructiveChangeCheckerFlavour {
    /// Check for potential destructive or unexecutable alter column steps.
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &ColumnDiffer<'_>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    );

    /// Check a DropAndRecreateColumn step.
    fn check_drop_and_recreate_column(
        &self,
        columns: &ColumnDiffer<'_>,
        changes: ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    );
}

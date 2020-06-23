mod mysql;
mod postgres;
mod sqlite;

use super::DestructiveCheckPlan;
use crate::sql_schema_differ::ColumnDiffer;
use sql_schema_describer::Table;

/// Flavour-specific destructive change checks.
pub(crate) trait DestructiveChangeCheckerFlavour {
    /// Check for potential destructive or unexecutable alter column steps.
    fn check_alter_column(&self, previous_table: &Table, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan);
}

mod mssql;
mod mysql;
mod postgres;
mod sqlite;

use sql_schema_describer::walkers::ColumnWalker;

use super::DestructiveCheckPlan;
use crate::{pair::Pair, sql_migration::AlterColumn, sql_schema_differ::ColumnChanges};

/// Flavour-specific destructive change checks.
pub(crate) trait DestructiveChangeCheckerFlavour {
    /// Check for potential destructive or unexecutable alter column steps.
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &Pair<ColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    );

    /// Check a DropAndRecreateColumn step.
    fn check_drop_and_recreate_column(
        &self,
        columns: &Pair<ColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    );
}

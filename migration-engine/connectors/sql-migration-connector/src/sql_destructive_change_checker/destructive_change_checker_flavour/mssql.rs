use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MssqlFlavour, sql_destructive_change_checker::destructive_check_plan::DestructiveCheckPlan,
    sql_migration::AlterColumn, sql_schema_differ::ColumnChanges, sql_schema_differ::ColumnDiffer,
};

impl DestructiveChangeCheckerFlavour for MssqlFlavour {
    fn check_alter_column(
        &self,
        _alter_column: &AlterColumn,
        _columns: &ColumnDiffer<'_>,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        todo!("check_alter_column on MSSQL")
    }

    fn check_drop_and_recreate_column(
        &self,
        _columns: &ColumnDiffer<'_>,
        _changes: ColumnChanges,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        panic!("check_drop_and_recreate_column on MSSQL")
    }
}

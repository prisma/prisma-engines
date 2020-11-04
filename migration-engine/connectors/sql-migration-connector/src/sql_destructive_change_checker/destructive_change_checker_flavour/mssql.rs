use sql_schema_describer::walkers::ColumnWalker;

use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MssqlFlavour, pair::Pair, sql_destructive_change_checker::destructive_check_plan::DestructiveCheckPlan,
    sql_migration::AlterColumn, sql_schema_differ::ColumnChanges,
};

impl DestructiveChangeCheckerFlavour for MssqlFlavour {
    fn check_alter_column(
        &self,
        _alter_column: &AlterColumn,
        _columns: &Pair<ColumnWalker<'_>>,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        todo!("check_alter_column on MSSQL")
    }

    fn check_drop_and_recreate_column(
        &self,
        _columns: &Pair<ColumnWalker<'_>>,
        _changes: &ColumnChanges,
        _plan: &mut DestructiveCheckPlan,
        _step_index: usize,
    ) {
        panic!("check_drop_and_recreate_column on MSSQL")
    }
}

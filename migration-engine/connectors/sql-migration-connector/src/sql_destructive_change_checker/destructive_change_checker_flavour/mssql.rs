use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MssqlFlavour, sql_destructive_change_checker::destructive_check_plan::DestructiveCheckPlan,
    sql_schema_differ::ColumnDiffer,
};

impl DestructiveChangeCheckerFlavour for MssqlFlavour {
    fn check_alter_column(&self, _: &ColumnDiffer<'_>, _: &mut DestructiveCheckPlan, _: usize) {
        todo!("check_alter_column on MSSQL");
    }
}

use super::DestructiveChangeCheckerFlavour;
use crate::{
    expanded_alter_column::{expand_mysql_alter_column, MysqlAlterColumn},
    flavour::MysqlFlavour,
    sql_destructive_changes_checker::{
        destructive_check_plan::DestructiveCheckPlan, warning_check::SqlMigrationWarning,
    },
    sql_schema_differ::ColumnDiffer,
};
use sql_schema_describer::Table;

impl DestructiveChangeCheckerFlavour for MysqlFlavour {
    fn check_alter_column(&self, previous_table: &Table, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        if let Some(steps) = expand_mysql_alter_column(columns) {
            for step in steps {
                match step {
                    MysqlAlterColumn::DropDefault | MysqlAlterColumn::SetDefault(_) => return,
                }
            }
        }

        plan.push_warning(SqlMigrationWarning::AlterColumn {
            table: previous_table.name.clone(),
            column: columns.next.name.clone(),
        });
    }
}

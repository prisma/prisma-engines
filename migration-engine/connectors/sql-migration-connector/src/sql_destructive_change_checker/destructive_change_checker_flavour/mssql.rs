use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::{MssqlFlavour, SqlFlavour},
    pair::Pair,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use sql_schema_describer::walkers::ColumnWalker;

#[async_trait::async_trait]
impl DestructiveChangeCheckerFlavour for MssqlFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &Pair<ColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let AlterColumn {
            column_id: _,
            changes,
            type_change,
        } = alter_column;

        if changes.only_default_changed() {
            return;
        }

        if changes.arity_changed() && columns.next().arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            );

            return;
        }

        match type_change {
            Some(ColumnTypeChange::SafeCast) | None => (),
            Some(ColumnTypeChange::RiskyCast) => {
                let previous_type = super::display_column_type(columns.previous, self.datamodel_connector());
                let next_type = super::display_column_type(columns.next, self.datamodel_connector());

                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous().table().name().to_owned(),
                        column: columns.previous().name().to_owned(),
                        previous_type,
                        next_type,
                    },
                    step_index,
                );
            }
            Some(ColumnTypeChange::NotCastable) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::NotCastable {
                        table: columns.previous().table().name().to_owned(),
                        column: columns.previous().name().to_owned(),
                        previous_type: format!("{:?}", columns.previous().column_type_family()),
                        next_type: format!("{:?}", columns.next().column_type_family()),
                    },
                    step_index,
                );
            }
        };
    }

    fn check_drop_and_recreate_column(
        &self,
        columns: &Pair<ColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        // Unexecutable drop and recreate.
        if changes.arity_changed()
            && columns.previous().arity().is_nullable()
            && columns.next().arity().is_required()
            && columns.next().default().is_none()
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::AddedRequiredFieldToTable {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        } else if columns.next().arity().is_required() && columns.next().default().is_none() {
            plan.push_unexecutable(
                UnexecutableStepCheck::DropAndRecreateRequiredColumn {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        } else {
            plan.push_warning(
                SqlMigrationWarningCheck::DropAndRecreateColumn {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        }
    }

    async fn count_rows_in_table(&mut self, table_name: &str) -> migration_connector::ConnectorResult<i64> {
        let query = {
            let schema_name = self.schema_name();
            format!("SELECT COUNT(*) FROM [{}].[{}]", schema_name, table_name)
        };
        let result_set = self.query_raw(&query, &[]).await?;
        super::extract_table_rows_count(table_name, result_set)
    }

    async fn count_values_in_column(
        &mut self,
        (table, column): (&str, &str),
    ) -> migration_connector::ConnectorResult<i64> {
        let query = {
            let schema_name = self.schema_name();
            format!(
                "SELECT COUNT(*) FROM [{}].[{}] WHERE [{}] IS NOT NULL",
                schema_name, table, column
            )
        };
        let result_set = self.query_raw(&query, &[]).await?;
        super::extract_column_values_count(result_set)
    }
}

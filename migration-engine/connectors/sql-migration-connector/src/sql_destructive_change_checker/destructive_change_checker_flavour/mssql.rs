use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::MssqlFlavour,
    pair::Pair,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use datamodel_connector::Connector;
use sql_datamodel_connector::SqlDatamodelConnectors;
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
                let datamodel_connector = SqlDatamodelConnectors::mssql(Default::default());
                let previous_type = match &columns.previous().column_type().native_type {
                    Some(tpe) => datamodel_connector.render_native_type(tpe.clone()),
                    _ => format!("{:?}", columns.previous().column_type_family()),
                };

                let next_type = match &columns.next().column_type().native_type {
                    Some(tpe) => datamodel_connector.render_native_type(tpe.clone()),
                    _ => format!("{:?}", columns.next().column_type_family()),
                };

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

    async fn count_rows_in_table(
        &self,
        table_name: &str,
        conn: &crate::connection_wrapper::Connection,
    ) -> migration_connector::ConnectorResult<i64> {
        let schema_name = conn.connection_info().schema_name();
        let query = format!("SELECT COUNT(*) FROM [{}].[{}]", schema_name, table_name);
        let result_set = conn.query_raw(&query, &[]).await?;
        super::extract_table_rows_count(table_name, result_set)
    }

    async fn count_values_in_column(
        &self,
        (table, column): (&str, &str),
        conn: &crate::connection_wrapper::Connection,
    ) -> migration_connector::ConnectorResult<i64> {
        let schema_name = conn.connection_info().schema_name();
        let query = format!(
            "SELECT COUNT(*) FROM [{}].[{}] WHERE [{}] IS NOT NULL",
            schema_name, table, column
        );
        let result_set = conn.query_raw(&query, &[]).await?;
        super::extract_column_values_count(result_set)
    }
}

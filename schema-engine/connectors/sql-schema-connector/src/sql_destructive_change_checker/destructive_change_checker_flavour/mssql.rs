use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::SqlConnector,
    migration_pair::MigrationPair,
    sql_destructive_change_checker::{
        check::{Column, Table},
        destructive_check_plan::DestructiveCheckPlan,
        unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use schema_connector::{BoxFuture, ConnectorResult};
use sql_schema_describer::walkers::TableColumnWalker;

#[derive(Debug, Default)]
pub struct MssqlDestructiveChangeCheckerFlavour {
    schema_name: String,
}

impl MssqlDestructiveChangeCheckerFlavour {
    pub fn new(schema_name: String) -> Self {
        Self { schema_name }
    }

    fn schema_name(&self) -> &str {
        &self.schema_name
    }

    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        psl::builtin_connectors::MSSQL
    }
}

impl DestructiveChangeCheckerFlavour for MssqlDestructiveChangeCheckerFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &MigrationPair<TableColumnWalker<'_>>,
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

        if changes.arity_changed() && columns.next.arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
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
                        table: columns.previous.table().name().to_owned(),
                        namespace: None,
                        column: columns.previous.name().to_owned(),
                        previous_type,
                        next_type,
                    },
                    step_index,
                );
            }
            Some(ColumnTypeChange::NotCastable) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::NotCastable {
                        table: columns.previous.table().name().to_owned(),
                        namespace: None,
                        column: columns.previous.name().to_owned(),
                        previous_type: format!("{:?}", columns.previous.column_type_family()),
                        next_type: format!("{:?}", columns.next.column_type_family()),
                    },
                    step_index,
                );
            }
        };
    }

    fn check_drop_and_recreate_column(
        &self,
        columns: &MigrationPair<TableColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        // Unexecutable drop and recreate.
        if changes.arity_changed()
            && columns.previous.arity().is_nullable()
            && columns.next.arity().is_required()
            && columns.next.default().is_none()
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::AddedRequiredFieldToTable(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            )
        } else if columns.next.arity().is_required() && columns.next.default().is_none() {
            plan.push_unexecutable(
                UnexecutableStepCheck::DropAndRecreateRequiredColumn(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            )
        } else {
            plan.push_warning(
                SqlMigrationWarningCheck::DropAndRecreateColumn {
                    column: columns.previous.name().to_owned(),
                    table: columns.previous.table().name().to_owned(),
                    namespace: None,
                },
                step_index,
            )
        }
    }

    fn count_rows_in_table<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        table: &'a Table,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let query = {
                let schema_name = table.namespace.as_deref().unwrap_or_else(|| self.schema_name());
                format!("SELECT COUNT(*) FROM [{}].[{}]", schema_name, table.table)
            };
            let result_set = connector.query_raw(&query, &[]).await?;
            super::extract_table_rows_count(table, result_set)
        })
    }

    fn count_values_in_column<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        column: &'a Column,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let query = {
                let schema_name = column.namespace.as_deref().unwrap_or_else(|| self.schema_name());
                format!(
                    "SELECT COUNT(*) FROM [{}].[{}] WHERE [{}] IS NOT NULL",
                    schema_name, column.table, column.column
                )
            };
            let result_set = connector.query_raw(&query, &[]).await?;
            super::extract_column_values_count(result_set)
        })
    }
}

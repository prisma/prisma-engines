use crate::{
    flavour::{postgres::Circumstances, SqlConnector},
    migration_pair::MigrationPair,
    sql_destructive_change_checker::{
        check::{Column, Table},
        destructive_change_checker_flavour::{
            display_column_type, extract_column_values_count, extract_table_rows_count,
        },
        destructive_check_plan::DestructiveCheckPlan,
        unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
        DestructiveChangeCheckerFlavour,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use enumflags2::BitFlags;
use schema_connector::{BoxFuture, ConnectorResult};
use sql_schema_describer::walkers::TableColumnWalker;

#[derive(Debug)]
pub struct PostgresDestructiveChangeCheckerFlavour {
    circumstances: BitFlags<Circumstances>,
}

impl PostgresDestructiveChangeCheckerFlavour {
    pub fn new(circumstances: BitFlags<Circumstances>) -> Self {
        Self { circumstances }
    }

    fn datamodel_connector(&self) -> &dyn psl::datamodel_connector::Connector {
        if self.circumstances.contains(Circumstances::IsCockroachDb) {
            psl::builtin_connectors::COCKROACH
        } else {
            psl::builtin_connectors::POSTGRES
        }
    }
}

impl DestructiveChangeCheckerFlavour for PostgresDestructiveChangeCheckerFlavour {
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

        if changes.arity_changed() && columns.previous.arity().is_nullable() && columns.next.arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            )
        }

        if changes.arity_changed() && !columns.previous.arity().is_list() && columns.next.arity().is_list() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeScalarFieldIntoArrayField(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            )
        }

        let previous_type = display_column_type(columns.previous, self.datamodel_connector());
        let next_type = display_column_type(columns.next, self.datamodel_connector());

        match type_change {
            None | Some(ColumnTypeChange::SafeCast) => (),
            Some(ColumnTypeChange::RiskyCast) => {
                plan.push_warning(
                    SqlMigrationWarningCheck::RiskyCast {
                        table: columns.previous.table().name().to_owned(),
                        namespace: columns.previous.table().namespace().map(String::from),
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
                        namespace: columns.previous.table().namespace().map(String::from),
                        column: columns.previous.name().to_owned(),
                        previous_type,
                        next_type,
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
            // todo this is probably due to a not castable type change. we should give that info in the warning
            plan.push_warning(
                SqlMigrationWarningCheck::DropAndRecreateColumn {
                    column: columns.previous.name().to_owned(),
                    namespace: columns.previous.table().namespace().map(String::from),
                    table: columns.previous.table().name().to_owned(),
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
            let from = match &table.namespace {
                Some(namespace) => format!("\"{}\".\"{}\"", namespace, table.table),
                None => format!("\"{}\"", table.table),
            };
            let query = format!("SELECT COUNT(*) FROM {from}");
            let result_set = connector.query_raw(&query, &[]).await?;
            extract_table_rows_count(table, result_set)
        })
    }

    fn count_values_in_column<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        column: &'a Column,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let from = match &column.namespace {
                Some(namespace) => format!("\"{}\".\"{}\"", namespace, column.table),
                None => format!("\"{}\"", column.table),
            };
            let query = format!("SELECT COUNT(*) FROM {} WHERE \"{}\" IS NOT NULL", from, column.column);
            let result_set = connector.query_raw(&query, &[]).await?;
            extract_column_values_count(result_set)
        })
    }
}

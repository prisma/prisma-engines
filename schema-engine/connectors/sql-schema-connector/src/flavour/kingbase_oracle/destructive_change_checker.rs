use crate::{
    flavour::SqlConnector,
    migration_pair::MigrationPair,
    sql_destructive_change_checker::{
        DestructiveChangeCheckerFlavour,
        check::{Column, Table},
        destructive_change_checker_flavour::{
            display_column_type, extract_column_values_count, extract_table_rows_count,
        },
        destructive_check_plan::DestructiveCheckPlan,
        unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{AlterColumn, ColumnTypeChange},
    sql_schema_differ::ColumnChanges,
};
use schema_connector::{BoxFuture, ConnectorResult};
use sql_schema_describer::walkers::TableColumnWalker;

/// Destructive-change diagnostics for KingbaseES Oracle mode.
#[derive(Debug, Default)]
pub(crate) struct KingbaseOracleDestructiveChangeCheckerFlavour;

impl DestructiveChangeCheckerFlavour for KingbaseOracleDestructiveChangeCheckerFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &MigrationPair<TableColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let AlterColumn {
            changes, type_change, ..
        } = alter_column;

        if changes.arity_changed() && columns.previous.arity().is_nullable() && columns.next.arity().is_required() {
            plan.push_unexecutable(
                UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                }),
                step_index,
            );
        }

        let previous_type = display_column_type(columns.previous, psl::builtin_connectors::KINGBASE_ORACLE);
        let next_type = display_column_type(columns.next, psl::builtin_connectors::KINGBASE_ORACLE);

        match type_change {
            None | Some(ColumnTypeChange::SafeCast) => (),
            Some(ColumnTypeChange::RiskyCast) => plan.push_warning(
                SqlMigrationWarningCheck::RiskyCast {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                    previous_type,
                    next_type,
                },
                step_index,
            ),
            Some(ColumnTypeChange::NotCastable) => plan.push_warning(
                SqlMigrationWarningCheck::NotCastable {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                    previous_type,
                    next_type,
                },
                step_index,
            ),
        }
    }

    fn check_drop_and_recreate_column(
        &self,
        columns: &MigrationPair<TableColumnWalker<'_>>,
        changes: &ColumnChanges,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let column = Column {
            table: columns.previous.table().name().to_owned(),
            namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
            column: columns.previous.name().to_owned(),
        };

        if changes.arity_changed()
            && columns.previous.arity().is_nullable()
            && columns.next.arity().is_required()
            && columns.next.default().is_none()
        {
            plan.push_unexecutable(UnexecutableStepCheck::AddedRequiredFieldToTable(column), step_index);
        } else if columns.next.arity().is_required() && columns.next.default().is_none() {
            plan.push_unexecutable(UnexecutableStepCheck::DropAndRecreateRequiredColumn(column), step_index);
        } else {
            plan.push_warning(
                SqlMigrationWarningCheck::DropAndRecreateColumn {
                    table: columns.previous.table().name().to_owned(),
                    namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                    column: columns.previous.name().to_owned(),
                },
                step_index,
            );
        }
    }

    fn count_rows_in_table<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        table: &'a Table,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let table_name = quoted_table_name(table.namespace.as_deref(), &table.table);
            extract_table_rows_count(
                table,
                connector
                    .query_raw(&format!("SELECT COUNT(*) FROM {table_name}"), &[])
                    .await?,
            )
        })
    }

    fn count_values_in_column<'a>(
        &'a mut self,
        connector: &'a mut dyn SqlConnector,
        column: &'a Column,
    ) -> BoxFuture<'a, ConnectorResult<i64>> {
        Box::pin(async move {
            let table_name = quoted_table_name(column.namespace.as_deref(), &column.table);
            let query = format!(
                "SELECT COUNT(*) FROM {table_name} WHERE {} IS NOT NULL",
                quote_identifier(&column.column)
            );
            extract_column_values_count(connector.query_raw(&query, &[]).await?)
        })
    }
}

fn quote_identifier(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('\"', "\"\""))
}

fn quoted_table_name(namespace: Option<&str>, table: &str) -> String {
    match namespace {
        Some(namespace) => format!("{}.{}", quote_identifier(namespace), quote_identifier(table)),
        None => quote_identifier(table),
    }
}

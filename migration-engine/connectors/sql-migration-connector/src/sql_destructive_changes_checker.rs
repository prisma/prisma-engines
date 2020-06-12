mod check;
mod database_inspection_results;
mod destructive_check_plan;
mod unexecutable_step_check;
mod warning_check;

use crate::{
    sql_schema_differ::DiffingOptions, AddColumn, AlterColumn, Component, DropColumn, DropTable, SqlMigration,
    SqlMigrationStep, SqlResult, TableChange,
};
use destructive_check_plan::DestructiveCheckPlan;
use migration_connector::{ConnectorResult, DestructiveChangeDiagnostics, DestructiveChangesChecker};
use quaint::prelude::SqlFamily;
use sql_schema_describer::{ColumnArity, SqlSchema};
use unexecutable_step_check::UnexecutableStepCheck;
use warning_check::SqlMigrationWarning;

/// The SqlDestructiveChangesChecker is responsible for informing users about potentially
/// destructive or impossible changes that their attempted migrations contain.
///
/// It proceeds in three steps:
///
/// - Examine the SqlMigrationSteps in the migration, to generate a `DestructiveCheckPlan`
///   containing destructive change checks (implementors of the `Check` trait). At this stage, there
///   is no interaction with the database.
/// - Execute that plan (`DestructiveCheckPlan::execute`), running queries against the database to
///   inspect its current state, depending on what information the checks require.
/// - Render the final user-facing messages based on the plan and the gathered information.
pub struct SqlDestructiveChangesChecker<'a> {
    pub connector: &'a crate::SqlMigrationConnector,
}

impl Component for SqlDestructiveChangesChecker<'_> {
    fn connector(&self) -> &crate::SqlMigrationConnector {
        self.connector
    }
}

impl SqlDestructiveChangesChecker<'_> {
    fn check_table_drop(&self, table_name: &str, plan: &mut DestructiveCheckPlan) {
        plan.push_warning(SqlMigrationWarning::NonEmptyTableDrop {
            table: table_name.to_owned(),
        });
    }

    /// Emit a warning when we drop a column that contains non-null values.
    fn check_column_drop(
        &self,
        drop_column: &DropColumn,
        table: &sql_schema_describer::Table,
        plan: &mut DestructiveCheckPlan,
    ) {
        plan.push_warning(SqlMigrationWarning::NonEmptyColumnDrop {
            table: table.name.clone(),
            column: drop_column.name.clone(),
        });
    }

    /// Columns cannot be added when all of the following holds:
    ///
    /// - There are existing rows
    /// - The new column is required
    /// - There is no default value for the new column
    fn check_add_column(
        &self,
        add_column: &AddColumn,
        table: &sql_schema_describer::Table,
        plan: &mut DestructiveCheckPlan,
    ) {
        let column_is_required_without_default =
            add_column.column.tpe.arity.is_required() && add_column.column.default.is_none();

        // Optional columns and columns with a default can safely be added.
        if !column_is_required_without_default {
            return;
        }

        let typed_unexecutable = UnexecutableStepCheck::AddedRequiredFieldToTable {
            column: add_column.column.name.clone(),
            table: table.name.clone(),
        };

        plan.push_unexecutable(typed_unexecutable);
    }

    /// Are considered safe at the moment:
    ///
    /// - renamings on SQLite
    /// - default changes on SQLite
    /// - Arity changes from required to optional on SQLite
    ///
    /// Are considered unexecutable:
    ///
    /// - Making an optional column required without a default, when there are existing rows in the table.
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        previous_table: &sql_schema_describer::Table,
        plan: &mut DestructiveCheckPlan,
    ) {
        let previous_column = previous_table
            .column(&alter_column.name)
            .expect("unsupported column renaming");

        let diffing_options = DiffingOptions::from_database_info(self.database_info());

        let differ = crate::sql_schema_differ::ColumnDiffer {
            diffing_options: &diffing_options,
            previous: previous_column,
            next: &alter_column.column,
        };

        if self.alter_column_is_safe(&differ) {
            return;
        }

        self.check_for_column_arity_change(&previous_table.name, &differ, plan);

        plan.push_warning(SqlMigrationWarning::AlterColumn {
            table: previous_table.name.clone(),
            column: alter_column.column.name.clone(),
        });

        if previous_table.is_part_of_foreign_key(&alter_column.column.name)
            && alter_column.column.default.is_none()
            && previous_column.default.is_some()
        {
            plan.push_warning(SqlMigrationWarning::ForeignKeyDefaultValueRemoved {
                table: previous_table.name.clone(),
                column: alter_column.name.clone(),
            });
        }
    }

    fn alter_column_is_safe(&self, differ: &crate::sql_schema_differ::ColumnDiffer<'_>) -> bool {
        use crate::sql_migration::expanded_alter_column::*;

        match self.sql_family() {
            SqlFamily::Sqlite => {
                let arity_change_is_safe = match (&differ.previous.tpe.arity, &differ.next.tpe.arity) {
                    // column became required
                    (ColumnArity::Nullable, ColumnArity::Required) => false,
                    // column became nullable
                    (ColumnArity::Required, ColumnArity::Nullable) => true,
                    // nothing changed
                    (ColumnArity::Required, ColumnArity::Required) | (ColumnArity::Nullable, ColumnArity::Nullable) => {
                        true
                    }
                    // not supported on SQLite
                    (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
                };

                !differ.all_changes().type_changed() && arity_change_is_safe
            }
            SqlFamily::Postgres => {
                let expanded = expand_postgres_alter_column(differ);

                // We keep the match here to keep the exhaustiveness checking for when we add variants.
                if let Some(steps) = expanded {
                    let mut is_safe = true;

                    for step in steps {
                        match step {
                            PostgresAlterColumn::SetDefault(_)
                            | PostgresAlterColumn::DropDefault
                            | PostgresAlterColumn::DropNotNull => (),
                            PostgresAlterColumn::SetType(_) => is_safe = false,
                        }
                    }

                    is_safe
                } else {
                    false
                }
            }
            SqlFamily::Mysql => {
                let expanded = expand_mysql_alter_column(differ);

                // We keep the match here to keep the exhaustiveness checking for when we add variants.
                if let Some(steps) = expanded {
                    let is_safe = true;

                    for step in steps {
                        match step {
                            MysqlAlterColumn::SetDefault(_) | MysqlAlterColumn::DropDefault => (),
                        }
                    }

                    is_safe
                } else {
                    false
                }
            }
        }
    }

    fn check_for_column_arity_change(
        &self,
        table_name: &str,
        differ: &crate::sql_schema_differ::ColumnDiffer<'_>,
        plan: &mut DestructiveCheckPlan,
    ) {
        if !differ.all_changes().arity_changed()
            || !differ.next.tpe.arity.is_required()
            || differ.next.default.is_some()
        {
            return;
        }

        let typed_unexecutable = unexecutable_step_check::UnexecutableStepCheck::MadeOptionalFieldRequired {
            table: table_name.to_owned(),
            column: differ.previous.name.clone(),
        };

        plan.push_unexecutable(typed_unexecutable);
    }

    #[tracing::instrument(skip(self, steps, before), target = "SqlDestructiveChangeChecker::check")]
    async fn check_impl(
        &self,
        steps: &[SqlMigrationStep],
        before: &SqlSchema,
    ) -> SqlResult<DestructiveChangeDiagnostics> {
        let mut plan = DestructiveCheckPlan::new();

        for step in steps {
            match step {
                SqlMigrationStep::AlterTable(alter_table) => {
                    // The table in alter_table is the updated table, but we want to
                    // check against the current state of the table.
                    let before_table = before.get_table(&alter_table.table.name);

                    if let Some(before_table) = before_table {
                        for change in &alter_table.changes {
                            match *change {
                                TableChange::DropColumn(ref drop_column) => {
                                    self.check_column_drop(drop_column, before_table, &mut plan)
                                }
                                TableChange::AlterColumn(ref alter_column) => {
                                    self.check_alter_column(alter_column, before_table, &mut plan)
                                }
                                TableChange::AddColumn(ref add_column) => {
                                    self.check_add_column(add_column, before_table, &mut plan)
                                }
                                _ => (),
                            }
                        }
                    }
                }
                // Here, check for each table we are going to delete if it is empty. If
                // not, return a warning.
                SqlMigrationStep::DropTable(DropTable { name }) => {
                    self.check_table_drop(name, &mut plan);
                }
                // SqlMigrationStep::CreateIndex(CreateIndex { table, index }) if index.is_unique() => todo!(),
                // do nothing
                _ => (),
            }
        }

        let mut diagnostics = plan.execute(self.schema_name(), self.conn()).await?;

        // Temporary, for better reporting.
        diagnostics.warn_about_unexecutable_migrations();

        Ok(diagnostics)
    }
}

#[async_trait::async_trait]
impl DestructiveChangesChecker<SqlMigration> for SqlDestructiveChangesChecker<'_> {
    async fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(&database_migration.original_steps, &database_migration.before)
            .await
            .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info()))
    }

    async fn check_unapply(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(&database_migration.rollback, &database_migration.after)
            .await
            .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info()))
    }
}

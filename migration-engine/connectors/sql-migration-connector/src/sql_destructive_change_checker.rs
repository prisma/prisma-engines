mod check;
mod database_inspection_results;
mod destructive_change_checker_flavour;
mod destructive_check_plan;
mod unexecutable_step_check;
mod warning_check;

pub(crate) use destructive_change_checker_flavour::DestructiveChangeCheckerFlavour;

use crate::{
    sql_schema_differ::{ColumnDiffer, DiffingOptions},
    sql_schema_helpers::SqlSchemaExt,
    AddColumn, Component, DropColumn, DropTable, SqlMigration, SqlMigrationStep, SqlResult, TableChange,
};
use destructive_check_plan::DestructiveCheckPlan;
use migration_connector::{ConnectorResult, DestructiveChangeChecker, DestructiveChangeDiagnostics};
use sql_schema_describer::SqlSchema;
use unexecutable_step_check::UnexecutableStepCheck;
use warning_check::SqlMigrationWarningCheck;

/// The SqlDestructiveChangeChecker is responsible for informing users about potentially
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
pub struct SqlDestructiveChangeChecker<'a> {
    pub connector: &'a crate::SqlMigrationConnector,
}

impl Component for SqlDestructiveChangeChecker<'_> {
    fn connector(&self) -> &crate::SqlMigrationConnector {
        self.connector
    }
}

impl SqlDestructiveChangeChecker<'_> {
    fn check_table_drop(&self, table_name: &str, plan: &mut DestructiveCheckPlan) {
        plan.push_warning(SqlMigrationWarningCheck::NonEmptyTableDrop {
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
        plan.push_warning(SqlMigrationWarningCheck::NonEmptyColumnDrop {
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

    fn plan(&self, steps: &[SqlMigrationStep], before: &SqlSchema, after: &SqlSchema) -> DestructiveCheckPlan {
        let mut plan = DestructiveCheckPlan::new();

        for step in steps {
            match step {
                SqlMigrationStep::AlterTable(alter_table) => {
                    // The table in alter_table is the updated table, but we want to
                    // check against the current state of the table.
                    let before_table = before.table_ref(&alter_table.table.name);
                    let after_table = after.table_ref(&alter_table.table.name);

                    if let (Some(before_table), Some(after_table)) = (before_table, after_table) {
                        for change in &alter_table.changes {
                            match *change {
                                TableChange::DropColumn(ref drop_column) => {
                                    self.check_column_drop(drop_column, &before_table.table, &mut plan)
                                }
                                TableChange::AlterColumn(ref alter_column) => {
                                    let previous_column = before_table
                                        .column(&alter_column.name)
                                        .expect("unsupported column renaming");
                                    let next_column = after_table
                                        .column(&alter_column.name)
                                        .expect("unsupported column renaming");

                                    let diffing_options = DiffingOptions::from_database_info(self.database_info());

                                    let differ = ColumnDiffer {
                                        diffing_options: &diffing_options,
                                        previous: previous_column,
                                        next: next_column,
                                    };

                                    self.flavour().check_alter_column(&differ, &mut plan)
                                }
                                TableChange::AddColumn(ref add_column) => {
                                    self.check_add_column(add_column, &before_table.table, &mut plan)
                                }
                                TableChange::DropPrimaryKey { .. } => {
                                    plan.push_warning(SqlMigrationWarningCheck::PrimaryKeyChange {
                                        table: alter_table.table.name.clone(),
                                    })
                                }
                                _ => (),
                            }
                        }
                    }
                }
                SqlMigrationStep::DropTable(DropTable { name }) => {
                    self.check_table_drop(name, &mut plan);
                }
                // SqlMigrationStep::CreateIndex(CreateIndex { table, index }) if index.is_unique() => todo!(),
                // do nothing
                _ => (),
            }
        }

        plan
    }

    #[tracing::instrument(skip(self, steps, before), target = "SqlDestructiveChangeChecker::check")]
    async fn check_impl(
        &self,
        steps: &[SqlMigrationStep],
        before: &SqlSchema,
        after: &SqlSchema,
    ) -> SqlResult<DestructiveChangeDiagnostics> {
        let plan = self.plan(steps, before, after);

        plan.execute(self.schema_name(), self.conn()).await
    }
}

#[async_trait::async_trait]
impl DestructiveChangeChecker<SqlMigration> for SqlDestructiveChangeChecker<'_> {
    async fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(
            &database_migration.original_steps,
            &database_migration.before,
            &database_migration.after,
        )
        .await
        .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info()))
    }

    async fn check_unapply(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        self.check_impl(
            &database_migration.rollback,
            &database_migration.after,
            &database_migration.before,
        )
        .await
        .map_err(|sql_error| sql_error.into_connector_error(&self.connection_info()))
    }

    fn pure_check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        let plan = self.plan(
            &database_migration.original_steps,
            &database_migration.before,
            &database_migration.after,
        );

        Ok(plan.pure_check())
    }
}

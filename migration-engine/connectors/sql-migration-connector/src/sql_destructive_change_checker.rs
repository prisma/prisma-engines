//! The SQL implementation of DestructiveChangeChecker is responsible for
//! informing users about potentially destructive or impossible changes that
//! their attempted migrations contain.
//!
//! It proceeds in three steps:
//!
//! - Examine the SqlMigrationSteps in the migration, to generate a
//!   `DestructiveCheckPlan` containing destructive change checks (implementors
//!   of the `Check` trait). At this stage, there is no interaction with the
//!   database.
//! - Execute that plan (`DestructiveCheckPlan::execute`), running queries
//!   against the database to inspect its current state, depending on what
//!   information the checks require.
//! - Render the final user-facing messages based on the plan and the gathered
//!   information.

mod check;
mod database_inspection_results;
mod destructive_change_checker_flavour;
mod destructive_check_plan;
mod unexecutable_step_check;
mod warning_check;

pub(crate) use destructive_change_checker_flavour::DestructiveChangeCheckerFlavour;

use crate::{
    pair::Pair,
    sql_migration::ColumnTypeChange,
    sql_migration::{AlterEnum, AlterTable, CreateIndex, DropTable, SqlMigrationStep, TableChange},
    SqlMigration, SqlMigrationConnector,
};
use destructive_check_plan::DestructiveCheckPlan;
use migration_connector::{ConnectorResult, DestructiveChangeChecker, DestructiveChangeDiagnostics};
use sql_schema_describer::{
    walkers::{ColumnWalker, SqlSchemaExt},
    ColumnArity, SqlSchema,
};
use unexecutable_step_check::UnexecutableStepCheck;
use warning_check::SqlMigrationWarningCheck;

impl SqlMigrationConnector {
    fn check_table_drop(&self, table_name: &str, plan: &mut DestructiveCheckPlan, step_index: usize) {
        plan.push_warning(
            SqlMigrationWarningCheck::NonEmptyTableDrop {
                table: table_name.to_owned(),
            },
            step_index,
        );
    }

    /// Emit a warning when we drop a column that contains non-null values.
    fn check_column_drop(&self, column: &ColumnWalker<'_>, plan: &mut DestructiveCheckPlan, step_index: usize) {
        plan.push_warning(
            SqlMigrationWarningCheck::NonEmptyColumnDrop {
                table: column.table().name().to_owned(),
                column: column.name().to_owned(),
            },
            step_index,
        );
    }

    /// Columns cannot be added when all of the following holds:
    ///
    /// - There are existing rows
    /// - The new column is required
    /// - There is no default value for the new column
    fn check_add_column(&self, column: &ColumnWalker<'_>, plan: &mut DestructiveCheckPlan, step_index: usize) {
        let column_is_required_without_default = column.arity().is_required() && column.default().is_none();

        // Optional columns and columns with a default can safely be added.
        if !column_is_required_without_default {
            return;
        }

        let typed_unexecutable = UnexecutableStepCheck::AddedRequiredFieldToTable {
            column: column.name().to_owned(),
            table: column.table().name().to_owned(),
        };

        plan.push_unexecutable(typed_unexecutable, step_index);
    }

    fn plan(&self, steps: &[SqlMigrationStep], schemas: &Pair<&SqlSchema>) -> DestructiveCheckPlan {
        let mut plan = DestructiveCheckPlan::new();

        for (step_index, step) in steps.iter().enumerate() {
            match step {
                SqlMigrationStep::AlterTable(AlterTable { table_index, changes }) => {
                    // The table in alter_table is the updated table, but we want to
                    // check against the current state of the table.
                    let tables = schemas.tables(table_index);

                    for change in changes {
                        match change {
                            TableChange::DropColumn(ref drop_column) => {
                                let column = tables.previous().column_at(drop_column.index);

                                self.check_column_drop(&column, &mut plan, step_index);
                            }
                            TableChange::AlterColumn(alter_column) => {
                                let columns = tables.columns(&alter_column.column_index);

                                self.flavour()
                                    .check_alter_column(&alter_column, &columns, &mut plan, step_index)
                            }
                            TableChange::AddColumn(ref add_column) => {
                                let column = tables.next().column_at(add_column.column_index);

                                self.check_add_column(&column, &mut plan, step_index)
                            }
                            TableChange::DropPrimaryKey { .. } => plan.push_warning(
                                SqlMigrationWarningCheck::PrimaryKeyChange {
                                    table: tables.previous().name().to_owned(),
                                },
                                step_index,
                            ),
                            TableChange::DropAndRecreateColumn { column_index, changes } => {
                                let columns = tables.columns(column_index);

                                self.flavour
                                    .check_drop_and_recreate_column(&columns, changes, &mut plan, step_index)
                            }
                            TableChange::AddPrimaryKey { .. } => (),
                        }
                    }
                }
                SqlMigrationStep::RedefineTables(redefine_tables) => {
                    for redefine_table in redefine_tables {
                        let tables = schemas.tables(&redefine_table.table_index);

                        if redefine_table.dropped_primary_key {
                            plan.push_warning(
                                SqlMigrationWarningCheck::PrimaryKeyChange {
                                    table: tables.previous().name().to_owned(),
                                },
                                step_index,
                            )
                        }

                        for added_column_idx in &redefine_table.added_columns {
                            let column = tables.next().column_at(*added_column_idx);
                            self.check_add_column(&column, &mut plan, step_index);
                        }

                        for dropped_column_idx in &redefine_table.dropped_columns {
                            let column = tables.previous().column_at(*dropped_column_idx);
                            self.check_column_drop(&column, &mut plan, step_index);
                        }

                        for (column_indexes, changes, type_change) in redefine_table.column_pairs.iter() {
                            let columns = tables.columns(column_indexes);

                            let arity_change_is_safe = match (&columns.previous().arity(), &columns.next().arity()) {
                                // column became required
                                (ColumnArity::Nullable, ColumnArity::Required) => false,
                                // column became nullable
                                (ColumnArity::Required, ColumnArity::Nullable) => true,
                                // nothing changed
                                (ColumnArity::Required, ColumnArity::Required)
                                | (ColumnArity::Nullable, ColumnArity::Nullable) => true,
                                // not supported on SQLite
                                (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
                            };

                            if !changes.type_changed() && arity_change_is_safe {
                                continue;
                            }

                            if changes.arity_changed()
                                && columns.next().arity().is_required()
                                && columns.next().default().is_none()
                            {
                                plan.push_unexecutable(
                                    UnexecutableStepCheck::MadeOptionalFieldRequired {
                                        table: columns.previous().table().name().to_owned(),
                                        column: columns.previous().name().to_owned(),
                                    },
                                    step_index,
                                );
                            }

                            match type_change {
                                Some(ColumnTypeChange::SafeCast) | None => (),
                                Some(ColumnTypeChange::RiskyCast) => {
                                    plan.push_warning(
                                        SqlMigrationWarningCheck::RiskyCast {
                                            table: columns.previous().table().name().to_owned(),
                                            column: columns.previous().name().to_owned(),
                                            previous_type: format!("{:?}", columns.previous().column_type_family()),
                                            next_type: format!("{:?}", columns.next().column_type_family()),
                                        },
                                        step_index,
                                    );
                                }
                            }
                        }
                    }
                }
                SqlMigrationStep::DropTable(DropTable { table_index }) => {
                    self.check_table_drop(
                        schemas.previous().table_walker_at(*table_index).name(),
                        &mut plan,
                        step_index,
                    );
                }
                SqlMigrationStep::CreateIndex(CreateIndex {
                    table,
                    index,
                    caused_by_create_table: false,
                }) if index.is_unique() => plan.push_warning(
                    SqlMigrationWarningCheck::UniqueConstraintAddition {
                        table: table.clone(),
                        columns: index.columns.clone(),
                    },
                    step_index,
                ),
                SqlMigrationStep::AlterEnum(AlterEnum {
                    index,
                    created_variants: _,
                    dropped_variants,
                }) if !dropped_variants.is_empty() => plan.push_warning(
                    SqlMigrationWarningCheck::EnumValueRemoval {
                        enm: schemas.next().enum_walker_at(*index.next()).name().to_owned(),
                        values: dropped_variants.clone(),
                    },
                    step_index,
                ),
                _ => (),
            }
        }

        plan
    }

    #[tracing::instrument(skip(self, steps, schemas), target = "SqlDestructiveChangeChecker::check")]
    async fn check_impl(
        &self,
        steps: &[SqlMigrationStep],
        schemas: &Pair<&SqlSchema>,
    ) -> ConnectorResult<DestructiveChangeDiagnostics> {
        let plan = self.plan(steps, schemas);

        plan.execute(self.conn()).await
    }
}

#[async_trait::async_trait]
impl DestructiveChangeChecker<SqlMigration> for SqlMigrationConnector {
    async fn check(&self, database_migration: &SqlMigration) -> ConnectorResult<DestructiveChangeDiagnostics> {
        let plan = self.plan(&database_migration.steps, &database_migration.schemas());

        plan.execute(self.conn()).await
    }

    fn pure_check(&self, database_migration: &SqlMigration) -> DestructiveChangeDiagnostics {
        let plan = self.plan(&database_migration.steps, &database_migration.schemas());

        plan.pure_check()
    }
}

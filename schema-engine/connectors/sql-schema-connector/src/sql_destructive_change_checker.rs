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

pub(crate) mod check;
mod database_inspection_results;
pub mod destructive_change_checker_flavour;
pub(crate) mod destructive_check_plan;
pub(crate) mod unexecutable_step_check;
pub(crate) mod warning_check;

pub(crate) use destructive_change_checker_flavour::DestructiveChangeCheckerFlavour;

use crate::{
    SqlMigration, SqlSchemaConnector,
    sql_migration::{AlterEnum, AlterTable, ColumnTypeChange, SqlMigrationStep, TableChange},
};
use destructive_check_plan::DestructiveCheckPlan;
use schema_connector::{BoxFuture, ConnectorResult, DestructiveChangeChecker, DestructiveChangeDiagnostics, Migration};
use sql_schema_describer::{ColumnArity, walkers::TableColumnWalker};
use unexecutable_step_check::UnexecutableStepCheck;
use warning_check::SqlMigrationWarningCheck;

use self::check::Column;

impl SqlSchemaConnector {
    fn check_table_drop(
        &self,
        table_name: &str,
        namespace: Option<&str>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        plan.push_warning(
            SqlMigrationWarningCheck::NonEmptyTableDrop {
                table: table_name.to_owned(),
                namespace: namespace.map(str::to_owned),
            },
            step_index,
        );
    }

    /// Emit a warning when we drop a column that contains non-null values.
    fn check_column_drop(&self, column: &TableColumnWalker<'_>, plan: &mut DestructiveCheckPlan, step_index: usize) {
        plan.push_warning(
            SqlMigrationWarningCheck::NonEmptyColumnDrop {
                table: column.table().name().to_owned(),
                namespace: column.table().explicit_namespace().map(str::to_owned),
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
    fn check_add_column(
        &self,
        column: &TableColumnWalker<'_>,
        has_virtual_default: bool,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let column_is_required_without_default = column.arity().is_required() && column.default().is_none();

        // Optional columns and columns with a default can safely be added.
        if !column_is_required_without_default {
            return;
        }

        let typed_unexecutable = if has_virtual_default {
            UnexecutableStepCheck::AddedRequiredFieldToTableWithPrismaLevelDefault(Column {
                table: column.table().name().to_owned(),
                namespace: column.table().explicit_namespace().map(str::to_owned),
                column: column.name().to_owned(),
            })
        } else {
            UnexecutableStepCheck::AddedRequiredFieldToTable(Column {
                table: column.table().name().to_owned(),
                namespace: column.table().explicit_namespace().map(str::to_owned),
                column: column.name().to_owned(),
            })
        };

        plan.push_unexecutable(typed_unexecutable, step_index);
    }

    fn plan(&self, migration: &SqlMigration) -> DestructiveCheckPlan {
        let steps = &migration.steps;
        let schemas = migration.schemas();
        let mut plan = DestructiveCheckPlan::new();
        let checker = self.sql_dialect().destructive_change_checker();

        for (step_index, step) in steps.iter().enumerate() {
            match step {
                SqlMigrationStep::AlterTable(AlterTable {
                    table_ids: table_id,
                    changes,
                }) => {
                    // The table in alter_table is the updated table, but we want to
                    // check against the current state of the table.
                    let tables = schemas.walk(*table_id);

                    for change in changes {
                        match change {
                            TableChange::DropColumn { column_id } => {
                                let column = schemas.previous.walk(*column_id);

                                self.check_column_drop(&column, &mut plan, step_index);
                            }
                            TableChange::AlterColumn(alter_column) => {
                                let columns = schemas.walk(alter_column.column_id);

                                checker.check_alter_column(alter_column, &columns, &mut plan, step_index)
                            }
                            TableChange::AddColumn {
                                column_id,
                                has_virtual_default,
                            } => {
                                let column = schemas.next.walk(*column_id);

                                self.check_add_column(&column, *has_virtual_default, &mut plan, step_index)
                            }
                            TableChange::DropPrimaryKey => plan.push_warning(
                                SqlMigrationWarningCheck::PrimaryKeyChange {
                                    table: tables.previous.name().to_owned(),
                                    namespace: tables.previous.explicit_namespace().map(str::to_owned),
                                },
                                step_index,
                            ),
                            TableChange::DropAndRecreateColumn { column_id, changes } => {
                                let columns = schemas.walk(*column_id);

                                checker.check_drop_and_recreate_column(&columns, changes, &mut plan, step_index)
                            }
                            TableChange::AddPrimaryKey => (),
                            TableChange::RenamePrimaryKey => (),
                        }
                    }
                }
                SqlMigrationStep::RedefineTables(redefine_tables) => {
                    for redefine_table in redefine_tables {
                        let tables = schemas.walk(redefine_table.table_ids);

                        if redefine_table.dropped_primary_key {
                            plan.push_warning(
                                SqlMigrationWarningCheck::PrimaryKeyChange {
                                    table: tables.previous.name().to_owned(),
                                    namespace: tables.previous.explicit_namespace().map(str::to_owned),
                                },
                                step_index,
                            )
                        }

                        for added_column_idx in &redefine_table.added_columns {
                            let column = schemas.next.walk(*added_column_idx);
                            let has_virtual_default = redefine_table
                                .added_columns_with_virtual_defaults
                                .contains(added_column_idx);
                            self.check_add_column(&column, has_virtual_default, &mut plan, step_index);
                        }

                        for dropped_column_idx in &redefine_table.dropped_columns {
                            let column = schemas.previous.walk(*dropped_column_idx);
                            self.check_column_drop(&column, &mut plan, step_index);
                        }

                        for (column_ides, changes, type_change) in redefine_table.column_pairs.iter() {
                            let columns = schemas.walk(*column_ides);

                            let arity_change_is_safe = match (&columns.previous.arity(), &columns.next.arity()) {
                                // column became required
                                (ColumnArity::Nullable, ColumnArity::Required) => false,
                                // column became nullable
                                (ColumnArity::Required, ColumnArity::Nullable) => true,
                                // nothing changed
                                (ColumnArity::Required, ColumnArity::Required)
                                | (ColumnArity::Nullable, ColumnArity::Nullable)
                                | (ColumnArity::List, ColumnArity::List) => true,
                                // not supported on SQLite
                                (ColumnArity::List, _) | (_, ColumnArity::List) => unreachable!(),
                            };

                            if !changes.type_changed() && arity_change_is_safe {
                                continue;
                            }

                            if changes.arity_changed()
                                && columns.next.arity().is_required()
                                && columns.next.default().is_none()
                            {
                                plan.push_unexecutable(
                                    UnexecutableStepCheck::MadeOptionalFieldRequired(Column {
                                        table: columns.previous.table().name().to_owned(),
                                        namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                                        column: columns.previous.name().to_owned(),
                                    }),
                                    step_index,
                                );
                            }

                            match type_change {
                                Some(ColumnTypeChange::SafeCast) | None => (),
                                Some(ColumnTypeChange::RiskyCast) => {
                                    plan.push_warning(
                                        SqlMigrationWarningCheck::RiskyCast {
                                            table: columns.previous.table().name().to_owned(),
                                            namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                                            column: columns.previous.name().to_owned(),
                                            previous_type: format!("{:?}", columns.previous.column_type_family()),
                                            next_type: format!("{:?}", columns.next.column_type_family()),
                                        },
                                        step_index,
                                    );
                                }
                                Some(ColumnTypeChange::NotCastable) => plan.push_warning(
                                    SqlMigrationWarningCheck::NotCastable {
                                        table: columns.previous.table().name().to_owned(),
                                        namespace: columns.previous.table().explicit_namespace().map(str::to_owned),
                                        column: columns.previous.name().to_owned(),
                                        previous_type: format!("{:?}", columns.previous.column_type_family()),
                                        next_type: format!("{:?}", columns.next.column_type_family()),
                                    },
                                    step_index,
                                ),
                            }
                        }
                    }
                }
                SqlMigrationStep::DropTable { table_id } => {
                    let table = schemas.previous.walk(*table_id);
                    self.check_table_drop(table.name(), table.explicit_namespace(), &mut plan, step_index);
                }
                SqlMigrationStep::CreateIndex {
                    table_id: (Some(_), _),
                    index_id,
                    from_drop_and_recreate: false,
                } => {
                    let index = schemas.next.walk(*index_id);
                    if index.is_unique() {
                        plan.push_warning(
                            SqlMigrationWarningCheck::UniqueConstraintAddition {
                                table: index.table().name().to_owned(),
                                columns: index.columns().map(|col| col.as_column().name().to_owned()).collect(),
                            },
                            step_index,
                        )
                    }
                }
                SqlMigrationStep::AlterEnum(AlterEnum {
                    id,
                    created_variants: _,
                    dropped_variants,
                    previous_usages_as_default: _,
                }) if !dropped_variants.is_empty() => plan.push_warning(
                    SqlMigrationWarningCheck::EnumValueRemoval {
                        enm: schemas.next.walk(id.next).name().to_owned(),
                        values: dropped_variants.clone(),
                    },
                    step_index,
                ),
                _ => (),
            }
        }

        plan
    }
}

impl DestructiveChangeChecker for SqlSchemaConnector {
    fn check<'a>(
        &'a mut self,
        migration: &'a Migration,
    ) -> BoxFuture<'a, ConnectorResult<DestructiveChangeDiagnostics>> {
        let plan = self.plan(migration.downcast_ref());
        Box::pin(async move { plan.execute(self.inner.as_mut()).await })
    }

    fn pure_check(&self, migration: &Migration) -> DestructiveChangeDiagnostics {
        let plan = self.plan(migration.downcast_ref());

        plan.pure_check()
    }
}

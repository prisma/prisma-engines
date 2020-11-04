use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::PostgresFlavour,
    pair::Pair,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::{
        expanded_alter_column::{expand_postgres_alter_column, PostgresAlterColumn},
        AlterColumn, ColumnTypeChange,
    },
    sql_schema_differ::ColumnChanges,
};
use sql_schema_describer::{walkers::ColumnWalker, ColumnArity, DefaultValue};

impl DestructiveChangeCheckerFlavour for PostgresFlavour {
    fn check_alter_column(
        &self,
        alter_column: &AlterColumn,
        columns: &Pair<ColumnWalker<'_>>,
        plan: &mut DestructiveCheckPlan,
        step_index: usize,
    ) {
        let AlterColumn {
            column_index: _,
            changes,
            type_change,
        } = alter_column;

        let steps = expand_postgres_alter_column(columns, changes);

        for step in steps {
            // We keep the match here to keep the exhaustiveness checking for when we add variants.
            match step {
                PostgresAlterColumn::SetNotNull => plan.push_unexecutable(
                    UnexecutableStepCheck::MadeOptionalFieldRequired {
                        column: columns.previous().name().to_owned(),
                        table: columns.previous().table().name().to_owned(),
                    },
                    step_index,
                ),
                PostgresAlterColumn::SetType(_) => {
                    if !matches!(columns.previous().arity(), ColumnArity::List)
                        && matches!(columns.next().arity(), ColumnArity::List)
                    {
                        plan.push_unexecutable(
                            UnexecutableStepCheck::MadeScalarFieldIntoArrayField {
                                table: columns.previous().table().name().to_owned(),
                                column: columns.previous().name().to_owned(),
                            },
                            step_index,
                        )
                    } else {
                        match type_change {
                            None => unreachable!("column_type_change is None on a Postgres SetType"),
                            Some(ColumnTypeChange::SafeCast) => (),
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
                        };
                    }
                }
                PostgresAlterColumn::SetDefault(_)
                | PostgresAlterColumn::AddSequence
                | PostgresAlterColumn::DropDefault
                | PostgresAlterColumn::DropNotNull => (),
            }
        }
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
            && !default_can_be_rendered(columns.next().default())
        {
            plan.push_unexecutable(
                UnexecutableStepCheck::AddedRequiredFieldToTable {
                    column: columns.previous().name().to_owned(),
                    table: columns.previous().table().name().to_owned(),
                },
                step_index,
            )
        } else {
            if columns.next().arity().is_required() && columns.next().default().is_none() {
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
    }
}

fn default_can_be_rendered(default: Option<&DefaultValue>) -> bool {
    match default {
        None => false,
        Some(DefaultValue::VALUE(_)) => true,
        Some(DefaultValue::DBGENERATED(expr)) => !expr.is_empty(),
        Some(DefaultValue::NOW) => true,
        Some(DefaultValue::SEQUENCE(_)) => false,
    }
}

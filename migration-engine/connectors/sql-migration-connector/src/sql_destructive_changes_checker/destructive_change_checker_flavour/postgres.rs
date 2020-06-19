use super::DestructiveChangeCheckerFlavour;
use crate::{
    expanded_alter_column::{expand_postgres_alter_column, PostgresAlterColumn},
    flavour::PostgresFlavour,
    sql_destructive_changes_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarning,
    },
    sql_schema_differ::ColumnDiffer,
};
use sql_schema_describer::{ColumnArity, DefaultValue, Table};

impl DestructiveChangeCheckerFlavour for PostgresFlavour {
    fn check_alter_column(&self, previous_table: &Table, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        let expanded = expand_postgres_alter_column(columns);

        if let Some(steps) = expanded {
            for step in steps {
                // We keep the match here to keep the exhaustiveness checking for when we add variants.
                match step {
                    PostgresAlterColumn::SetNotNull => {
                        plan.push_unexecutable(UnexecutableStepCheck::MadeOptionalFieldRequired {
                            column: columns.previous.name.clone(),
                            table: previous_table.name.clone(),
                        })
                    }
                    PostgresAlterColumn::SetType(_) => {
                        if !matches!(columns.previous.tpe.arity, ColumnArity::List)
                            && matches!(columns.next.tpe.arity, ColumnArity::List)
                        {
                            plan.push_unexecutable(UnexecutableStepCheck::MadeScalarFieldIntoArrayField {
                                table: previous_table.name.clone(),
                                column: columns.previous.name.clone(),
                            })
                        } else {
                            plan.push_warning(SqlMigrationWarning::AlterColumn {
                                table: previous_table.name.clone(),
                                column: columns.previous.name.clone(),
                            });
                        }
                    }
                    PostgresAlterColumn::SetDefault(_)
                    | PostgresAlterColumn::DropDefault
                    | PostgresAlterColumn::DropNotNull => (),
                }
            }
        } else {
            // Unexecutable drop and recreate.
            if columns.all_changes().arity_changed()
                && columns.previous.tpe.arity.is_nullable()
                && columns.next.tpe.arity.is_required()
                && !default_can_be_rendered(columns.next.default.as_ref())
            {
                plan.push_unexecutable(UnexecutableStepCheck::AddedRequiredFieldToTable {
                    column: columns.previous.name.clone(),
                    table: previous_table.name.clone(),
                })
            } else {
                // Executable drop and recreate.
                plan.push_warning(SqlMigrationWarning::AlterColumn {
                    table: previous_table.name.clone(),
                    column: columns.next.name.clone(),
                });
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

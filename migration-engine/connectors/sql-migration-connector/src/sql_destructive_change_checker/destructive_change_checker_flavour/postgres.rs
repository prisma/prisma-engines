use super::DestructiveChangeCheckerFlavour;
use crate::{
    flavour::PostgresFlavour,
    sql_destructive_change_checker::{
        destructive_check_plan::DestructiveCheckPlan, unexecutable_step_check::UnexecutableStepCheck,
        warning_check::SqlMigrationWarningCheck,
    },
    sql_migration::expanded_alter_column::{expand_postgres_alter_column, PostgresAlterColumn},
    sql_schema_differ::ColumnDiffer,
};
use sql_schema_describer::{ColumnArity, DefaultValue};

impl DestructiveChangeCheckerFlavour for PostgresFlavour {
    fn check_alter_column(&self, columns: &ColumnDiffer<'_>, plan: &mut DestructiveCheckPlan) {
        let expanded = expand_postgres_alter_column(columns);

        if let Some(steps) = expanded {
            for step in steps {
                // We keep the match here to keep the exhaustiveness checking for when we add variants.
                match step {
                    PostgresAlterColumn::SetNotNull => {
                        plan.push_unexecutable(UnexecutableStepCheck::MadeOptionalFieldRequired {
                            column: columns.previous.name().to_owned(),
                            table: columns.previous.table().name().to_owned(),
                        })
                    }
                    PostgresAlterColumn::SetType(_) => {
                        if !matches!(columns.previous.arity(), ColumnArity::List)
                            && matches!(columns.next.arity(), ColumnArity::List)
                        {
                            plan.push_unexecutable(UnexecutableStepCheck::MadeScalarFieldIntoArrayField {
                                table: columns.previous.table().name().to_owned(),
                                column: columns.previous.name().to_owned(),
                            })
                        } else {
                            plan.push_warning(SqlMigrationWarningCheck::AlterColumn {
                                table: columns.previous.table().name().to_owned(),
                                column: columns.previous.name().to_owned(),
                            });
                        }
                    }
                    PostgresAlterColumn::SetDefault(_)
                    | PostgresAlterColumn::AddSequence
                    | PostgresAlterColumn::DropDefault
                    | PostgresAlterColumn::DropNotNull => (),
                }
            }
        } else {
            // Unexecutable drop and recreate.
            if columns.all_changes().arity_changed()
                && columns.previous.arity().is_nullable()
                && columns.next.arity().is_required()
                && !default_can_be_rendered(columns.next.default())
            {
                plan.push_unexecutable(UnexecutableStepCheck::AddedRequiredFieldToTable {
                    column: columns.previous.name().to_owned(),
                    table: columns.previous.table().name().to_owned(),
                })
            } else {
                // Executable drop and recreate.
                plan.push_warning(SqlMigrationWarningCheck::AlterColumn {
                    table: columns.previous.table().name().to_owned(),
                    column: columns.next.name().to_owned(),
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

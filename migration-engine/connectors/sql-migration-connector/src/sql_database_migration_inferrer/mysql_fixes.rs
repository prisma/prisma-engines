use crate::{error::SqlResult, sql_migration::*};
use sql_schema_describer::{Column, ColumnArity, ColumnTypeFamily, Table};

pub(crate) fn fix_steps(steps: Vec<SqlMigrationStep>, schema_name: &str) -> SqlResult<Vec<SqlMigrationStep>> {
    let mut fixed_steps = Vec::with_capacity(steps.len());
    for step in steps {
        match step {
            SqlMigrationStep::AlterTable(alter_table) => {
                fix_mysql_new_required_text_fields(alter_table, &mut fixed_steps, schema_name)?
            }
            other_step => fixed_steps.push(other_step),
        }
    }

    Ok(fixed_steps)
}

/// On MySQL before version 8, `DEFAULT` on `TEXT` fields is not supported.
fn fix_mysql_new_required_text_fields(
    original_step: AlterTable,
    fixed_steps: &mut Vec<SqlMigrationStep>,
    schema_name: &str,
) -> SqlResult<()> {
    let mut other_changes = Vec::new();
    let mut added_required_text_columns = Vec::new();
    let table = &original_step.table;

    for change in original_step.changes {
        match change {
            TableChange::AddColumn(AddColumn { column }) if is_required_new_text_column(&column) => {
                added_required_text_columns.push(column)
            }
            change => other_changes.push(change),
        }
    }

    for column in added_required_text_columns {
        fix_mysql_add_required_text_column(table, column, fixed_steps, schema_name)?;
    }

    // Then push the changes that don't need fixing up.
    fixed_steps.push(SqlMigrationStep::AlterTable(AlterTable {
        table: original_step.table,
        changes: other_changes,
    }));

    Ok(())
}

fn fix_mysql_add_required_text_column(
    table: &Table,
    column: Column,
    fixed_steps: &mut Vec<SqlMigrationStep>,
    schema_name: &str,
) -> SqlResult<()> {
    // First add the column as non-required.
    let non_required_column = {
        let mut col = column.clone();
        col.arity = ColumnArity::Nullable;
        col
    };
    fixed_steps.push(SqlMigrationStep::AlterTable(AlterTable {
        table: table.clone(),
        changes: vec![TableChange::AddColumn(AddColumn {
            column: non_required_column,
        })],
    }));

    // Then update all columns to the default value if we have one. Otherwise assume the user knows what they are doing and the table is empty.
    if let Some(default_value) = column.default.as_ref() {
        let update_statement = format!(
            r#"UPDATE `{schema_name}`.`{table_name}` SET `{column_name}` = '{default_value}'"#,
            schema_name = schema_name,
            table_name = &table.name,
            column_name = &column.name,
            default_value = default_value
        );

        fixed_steps.push(SqlMigrationStep::RawSql { raw: update_statement });
    }

    // Finally, make the column NOT NULL.

    // let alter_table_statement = format!(
    //     r#"ALTER TABLE `{schema_name}`.`{table_name}` MODIFY `{column_name}` {} NOT NULL"#,
    //     schema_name = schema_name,
    //     table_name = &table.name,
    //     column_name = &column.name,
    //     column_type = MYSQL_STRING_COLUMN_TYPE,
    // );

    // fixed_steps.push(SqlMigrationStep::RawSql {
    //     raw: alter_table_statement,
    // });

    fixed_steps.push(SqlMigrationStep::AlterTable(AlterTable {
        table: table.clone(),
        changes: vec![TableChange::AlterColumn(AlterColumn {
            name: column.name.clone(),
            column,
            change: ColumnChange::ChangeArity {
                from: ColumnArity::Nullable,
                to: ColumnArity::Required,
            },
        })],
    }));

    Ok(())
}

fn is_required_new_text_column(column: &Column) -> bool {
    column.tpe.family == ColumnTypeFamily::String && column.arity == ColumnArity::Required
}

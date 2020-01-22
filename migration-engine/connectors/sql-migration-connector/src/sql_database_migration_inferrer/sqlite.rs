use crate::{
    sql_migration::*,
    sql_schema_differ::{ColumnDiffer, SqlSchemaDiff, TableDiffer},
    SqlResult,
};
use sql_schema_describer::{Column, ColumnArity, ColumnTypeFamily, SqlSchema, Table};

pub(super) fn fix(
    diff: SqlSchemaDiff,
    current_database_schema: &SqlSchema,
    next_database_schema: &SqlSchema,
    schema_name: &str,
) -> SqlResult<Vec<SqlMigrationStep>> {
    let steps = diff.into_steps();

    let mut result = Vec::new();
    let mut fixed_tables = Vec::new();

    for step in steps {
        match step {
            SqlMigrationStep::AlterTable(ref alter_table)
                if needs_fix(&alter_table) && current_database_schema.has_table(&alter_table.table.name) =>
            {
                result.extend(sqlite_fix_table(
                    current_database_schema,
                    next_database_schema,
                    &alter_table.table.name,
                    schema_name,
                )?);
                fixed_tables.push(alter_table.table.name.clone());
            }
            SqlMigrationStep::AddForeignKey(add_foreign_key) if fixed_tables.contains(&add_foreign_key.table) => {
                // The fixed alter table step will already create the foreign key.
            }
            SqlMigrationStep::CreateIndex(ref create_index) if fixed_tables.contains(&create_index.table) => {
                // The fixed alter table step will already create the index.
            }
            SqlMigrationStep::AlterIndex(AlterIndex { table, .. }) => {
                result.extend(sqlite_fix_table(
                    current_database_schema,
                    next_database_schema,
                    &table,
                    schema_name,
                )?);
                fixed_tables.push(table.clone());
            }
            x => result.push(x),
        }
    }

    Ok(result)
}

fn needs_fix(alter_table: &AlterTable) -> bool {
    let change_that_does_not_work_on_sqlite = alter_table.changes.iter().find(|change| match change {
        TableChange::AddColumn(add_column) => {
            // sqlite does not allow adding not null columns without a default value even if the table is empty
            // hence we just use our normal migration process
            // https://laracasts.com/discuss/channels/general-discussion/migrations-sqlite-general-error-1-cannot-add-a-not-null-column-with-default-value-null
            add_column.column.tpe.arity == ColumnArity::Required
        }
        TableChange::DropColumn(_) => true,
        TableChange::AlterColumn(_) => true,
        TableChange::DropForeignKey(_) => true,
    });

    change_that_does_not_work_on_sqlite.is_some()
}

fn sqlite_fix_table(
    current_database_schema: &SqlSchema,
    next_database_schema: &SqlSchema,
    table_name: &str,
    schema_name: &str,
) -> SqlResult<impl Iterator<Item = SqlMigrationStep>> {
    let current_table = current_database_schema.table(table_name)?;
    let next_table = next_database_schema.table(table_name)?;
    Ok(fix_table(&current_table, &next_table, &schema_name).into_iter())
}

fn fix_table(current: &Table, next: &Table, schema_name: &str) -> Vec<SqlMigrationStep> {
    // based on 'Making Other Kinds Of Table Schema Changes' from https://www.sqlite.org/lang_altertable.html
    let name_of_temporary_table = format!("new_{}", next.name.clone());
    let mut temporary_table = next.clone();
    temporary_table.name = name_of_temporary_table.clone();

    let mut result = Vec::new();

    result.push(SqlMigrationStep::RawSql {
        raw: "PRAGMA foreign_keys=OFF;".to_string(),
    });
    // todo: start transaction now. Unclear if we really want to do that.
    result.push(SqlMigrationStep::CreateTable(CreateTable {
        table: temporary_table.clone(),
    }));

    copy_current_table_into_new_table(
        &mut result,
        TableDiffer {
            previous: current,
            next: &temporary_table,
        },
        schema_name,
    )
    .unwrap();

    result.push(SqlMigrationStep::DropTable(DropTable {
        name: current.name.clone(),
    }));

    result.push(SqlMigrationStep::RenameTable {
        name: name_of_temporary_table,
        new_name: next.name.clone(),
    });

    // Recreate the indices
    result.extend(next.indices.iter().map(|index| {
        SqlMigrationStep::CreateIndex(CreateIndex {
            table: next.name.clone(),
            index: index.clone(),
        })
    }));

    result.push(SqlMigrationStep::RawSql {
        raw: format!(r#"PRAGMA "{}".foreign_key_check;"#, schema_name),
    });

    result.push(SqlMigrationStep::RawSql {
        raw: "PRAGMA foreign_keys=ON;".to_string(),
    });

    result
}

/// Copy the existing data into the new table.
///
/// The process is complicated by the migrations that add make an optional column required with a
/// default value. In this case, we need to treat them differently and `coalesce`ing them with the
/// default value, since SQLite does not have the `DEFAULT` keyword.
fn copy_current_table_into_new_table(
    steps: &mut Vec<SqlMigrationStep>,
    differ: TableDiffer<'_>,
    schema_name: &str,
) -> std::fmt::Result {
    use std::fmt::Write as _;
    let columns_that_became_required_with_a_default: Vec<ColumnDiffer<'_>> = differ
        .column_pairs()
        .filter(|columns| {
            columns.all_changes().arity_changed()
                && columns.next.tpe.arity.is_required()
                && columns.next.default.is_some()
        })
        .collect();
    let intersection_columns: Vec<String> = differ
        .column_pairs()
        .filter(|columns| {
            !columns_that_became_required_with_a_default
                .iter()
                .any(|excluded| excluded.name() == columns.name())
        })
        .map(|columns| format!(r#""{}""#, columns.name()))
        .collect();

    let mut query = String::with_capacity(40);

    write!(query, r#"INSERT INTO "{}"."{}" ("#, schema_name, &differ.next.name)?;

    let mut destination_columns = intersection_columns
        .iter()
        .map(|s| s.clone())
        .chain(
            columns_that_became_required_with_a_default
                .iter()
                .map(|columns| format!(r#""{}""#, columns.name())),
        )
        .peekable();

    while let Some(destination_column) = destination_columns.next() {
        write!(query, "{}", destination_column)?;

        if destination_columns.peek().is_some() {
            write!(query, ", ")?;
        }
    }

    write!(query, r#") SELECT "#)?;

    let mut source_columns = intersection_columns
        .iter()
        .map(|s| s.clone())
        .chain(columns_that_became_required_with_a_default.iter().map(|columns| {
            format!(
                r#"coalesce("{column_name}", {default_value}) AS "{column_name}""#,
                column_name = columns.name(),
                default_value = render_default(&columns.next)
            )
        }))
        .peekable();

    while let Some(source_column) = source_columns.next() {
        write!(query, "{}", source_column)?;

        if source_columns.peek().is_some() {
            write!(query, ", ")?;
        }
    }

    write!(query, r#" FROM "{}""#, &differ.previous.name)?;

    steps.push(SqlMigrationStep::RawSql { raw: query });

    Ok(())
}

fn render_default(column: &Column) -> String {
    match column.tpe.family {
        ColumnTypeFamily::String => format!("'{}'", column.default.as_ref().unwrap()),
        _ => column.default.as_ref().unwrap().to_string(),
    }
}

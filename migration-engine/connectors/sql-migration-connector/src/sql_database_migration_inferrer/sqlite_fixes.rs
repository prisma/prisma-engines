use crate::{error::SqlResult, sql_migration::*, sql_schema_differ::SqlSchemaDiff};
use sql_schema_describer::{ColumnArity, SqlSchema, Table};

pub(crate) fn fix_stupid_sqlite(
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
            SqlMigrationStep::AlterTable(ref alter_table) if needs_fix(&alter_table) => {
                result.extend(sqlite_fix_table(
                    current_database_schema,
                    next_database_schema,
                    &alter_table.table.name,
                    schema_name,
                )?);
                fixed_tables.push(alter_table.table.name.clone());
            }
            SqlMigrationStep::CreateIndex(ref create_index) if fixed_tables.contains(&create_index.table.name) => {
                // The fixed alter table step will already create the index
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

pub(crate) fn needs_fix(alter_table: &AlterTable) -> bool {
    let change_that_does_not_work_on_sqlite = alter_table.changes.iter().find(|change| match change {
        TableChange::AddColumn(add_column) => {
            // sqlite does not allow adding not null columns without a default value even if the table is empty
            // hence we just use our normal migration process
            // https://laracasts.com/discuss/channels/general-discussion/migrations-sqlite-general-error-1-cannot-add-a-not-null-column-with-default-value-null
            add_column.column.arity == ColumnArity::Required
        }
        TableChange::DropColumn(_) => true,
        TableChange::AlterColumn(_) => true,
    });
    change_that_does_not_work_on_sqlite.is_some()
}

pub(crate) fn sqlite_fix_table(
    current_database_schema: &SqlSchema,
    next_database_schema: &SqlSchema,
    table_name: &str,
    schema_name: &str,
) -> SqlResult<impl Iterator<Item = SqlMigrationStep>> {
    let current_table = current_database_schema.table(table_name)?;
    let next_table = next_database_schema.table(table_name)?;
    Ok(fix(&current_table, &next_table, &schema_name).into_iter())
}

fn fix(current: &Table, next: &Table, schema_name: &str) -> Vec<SqlMigrationStep> {
    // based on 'Making Other Kinds Of Table Schema Changes' from https://www.sqlite.org/lang_altertable.html
    let name_of_temporary_table = format!("new_{}", next.name.clone());
    let mut temporary_table = next.clone();
    temporary_table.name = name_of_temporary_table.clone();

    let mut result = Vec::new();

    result.push(SqlMigrationStep::RawSql {
        raw: "PRAGMA foreign_keys=OFF;".to_string(),
    });
    // todo: start transaction now. Unclear if we really want to do that.
    result.push(SqlMigrationStep::CreateTable(CreateTable { table: temporary_table }));
    result.push(
        // copy table contents; Here we have to handle escpaing ourselves.
        {
            let current_columns: Vec<String> = current.columns.iter().map(|c| c.name.clone()).collect();
            let next_columns: Vec<String> = next.columns.iter().map(|c| c.name.clone()).collect();
            let intersection_columns: Vec<String> = current_columns
                .into_iter()
                .filter(|c| next_columns.contains(&c))
                .collect();
            let columns_string = intersection_columns
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<String>>()
                .join(",");
            let sql = format!(
                "INSERT INTO \"{}\" ({}) SELECT {} from \"{}\"",
                name_of_temporary_table,
                columns_string,
                columns_string,
                next.name.clone()
            );
            SqlMigrationStep::RawSql { raw: sql.to_string() }
        },
    );

    result.push(SqlMigrationStep::DropTable(DropTable {
        name: current.name.clone(),
    }));
    result.push(SqlMigrationStep::RenameTable {
        name: name_of_temporary_table,
        new_name: next.name.clone(),
    });
    result.append(
        &mut next
            .indices
            .iter()
            .map(|index| {
                SqlMigrationStep::CreateIndex(CreateIndex {
                    table: next.clone(),
                    index: index.clone(),
                })
            })
            .collect(),
    );
    // todo: recreate triggers
    result.push(SqlMigrationStep::RawSql {
        raw: format!(r#"PRAGMA "{}".foreign_key_check;"#, schema_name),
    });
    // todo: commit transaction
    result.push(SqlMigrationStep::RawSql {
        raw: "PRAGMA foreign_keys=ON;".to_string(),
    });

    result
}

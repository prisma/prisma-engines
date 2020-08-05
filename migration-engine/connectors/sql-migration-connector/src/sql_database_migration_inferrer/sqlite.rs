use crate::{
    database_info::DatabaseInfo,
    flavour::SqlFlavour,
    sql_migration::*,
    sql_renderer::Quoted,
    sql_schema_differ::{ColumnDiffer, SqlSchemaDiff, TableDiffer},
    sql_schema_helpers::{SqlSchemaExt, TableRef},
    SqlResult,
};
use sql_schema_describer::{ColumnArity, SqlSchema};

pub(super) fn fix(
    diff: SqlSchemaDiff,
    current_database_schema: &SqlSchema,
    next_database_schema: &SqlSchema,
    schema_name: &str,
    database_info: &DatabaseInfo,
    flavour: &dyn SqlFlavour,
) -> SqlResult<Vec<SqlMigrationStep>> {
    let steps = diff.into_steps();

    let mut result = Vec::new();
    let mut fixed_tables = Vec::new();

    result.push(SqlMigrationStep::RawSql {
        raw: "PRAGMA foreign_keys=OFF".to_string(),
    });

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
                    database_info,
                    flavour,
                )?);
                fixed_tables.push(alter_table.table.name.clone());
            }
            SqlMigrationStep::AddForeignKey(add_foreign_key) if fixed_tables.contains(&add_foreign_key.table) => {
                // The fixed alter table step will already create the foreign key.
            }
            SqlMigrationStep::DropForeignKey(drop_foreign_key) if fixed_tables.contains(&drop_foreign_key.table) => {
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
                    database_info,
                    flavour,
                )?);
                fixed_tables.push(table.clone());
            }
            x => result.push(x),
        }
    }

    // No steps
    if result.len() == 1 {
        return Ok(Vec::new());
    }

    result.push(SqlMigrationStep::RawSql {
        raw: format!("PRAGMA {}.foreign_key_check", Quoted::sqlite_ident(schema_name)),
    });

    result.push(SqlMigrationStep::RawSql {
        raw: "PRAGMA foreign_keys=ON".to_string(),
    });

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
        TableChange::DropColumn(_)
        | TableChange::AlterColumn(_)
        | TableChange::DropPrimaryKey { .. }
        | TableChange::AddPrimaryKey { .. } => true,
    });

    change_that_does_not_work_on_sqlite.is_some()
}

fn sqlite_fix_table(
    current_database_schema: &SqlSchema,
    next_database_schema: &SqlSchema,
    table_name: &str,
    schema_name: &str,
    database_info: &DatabaseInfo,
    flavour: &dyn SqlFlavour,
) -> SqlResult<impl Iterator<Item = SqlMigrationStep>> {
    let current_table = current_database_schema
        .table_ref(table_name)
        .expect("SQLite table referenced in migration not found.");
    let next_table = next_database_schema
        .table_ref(table_name)
        .expect("SQLite table referenced in migration not found.");
    Ok(fix_table(current_table, next_table, &schema_name, database_info, flavour).into_iter())
}

fn fix_table(
    current: TableRef<'_>,
    next: TableRef<'_>,
    schema_name: &str,
    database_info: &DatabaseInfo,
    flavour: &dyn SqlFlavour,
) -> Vec<SqlMigrationStep> {
    // Based on 'Making Other Kinds Of Table Schema Changes' from https://www.sqlite.org/lang_altertable.html
    let name_of_temporary_table = format!("new_{}", &next.name());
    let mut temporary_table = next.table.clone();
    temporary_table.name = name_of_temporary_table.clone();

    let mut result = Vec::new();

    // todo: start transaction now. Unclear if we really want to do that.
    result.push(SqlMigrationStep::CreateTable(CreateTable {
        table: temporary_table.clone(),
    }));

    copy_current_table_into_new_table(
        &mut result,
        TableDiffer {
            flavour,
            database_info,
            previous: current,
            next: TableRef::new(next.schema, &temporary_table),
        },
        schema_name,
        flavour,
    )
    .unwrap();

    result.push(SqlMigrationStep::DropTable(DropTable {
        name: current.name().to_owned(),
    }));

    result.push(SqlMigrationStep::RenameTable {
        name: name_of_temporary_table,
        new_name: next.name().to_owned(),
    });

    // Recreate the indices
    result.extend(next.table.indices.iter().map(|index| {
        SqlMigrationStep::CreateIndex(CreateIndex {
            table: next.name().to_owned(),
            index: index.clone(),
        })
    }));

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
    flavour: &dyn SqlFlavour,
) -> std::fmt::Result {
    use std::fmt::Write as _;
    let columns_that_became_required_with_a_default: Vec<ColumnDiffer<'_>> = differ
        .column_pairs()
        .filter(|columns| {
            columns.all_changes().arity_changed()
                && columns.next.column.tpe.arity.is_required()
                && columns.next.column.default.is_some()
        })
        .collect();
    let intersection_columns: Vec<&str> = differ
        .column_pairs()
        .filter(|columns| {
            !columns_that_became_required_with_a_default
                .iter()
                .any(|excluded| excluded.name() == columns.name())
        })
        .map(|columns| columns.name())
        .collect();

    let mut query = String::with_capacity(40);

    write!(
        query,
        "INSERT INTO {}.{} (",
        Quoted::sqlite_ident(schema_name),
        Quoted::sqlite_ident(&differ.next.name())
    )?;

    let mut destination_columns = intersection_columns
        .iter()
        .copied()
        .chain(
            columns_that_became_required_with_a_default
                .iter()
                .map(|columns| columns.name()),
        )
        .peekable();

    while let Some(destination_column) = destination_columns.next() {
        write!(query, "{}", Quoted::sqlite_ident(destination_column))?;

        if destination_columns.peek().is_some() {
            write!(query, ", ")?;
        }
    }

    write!(query, r#") SELECT "#)?;

    let mut source_columns = intersection_columns
        .iter()
        .map(|s| format!("{}", Quoted::sqlite_ident(s)))
        .chain(columns_that_became_required_with_a_default.iter().map(|columns| {
            format!(
                "coalesce({column_name}, {default_value}) AS {column_name}",
                column_name = Quoted::sqlite_ident(columns.name()),
                default_value = flavour.render_default(
                    columns
                        .next
                        .column
                        .default
                        .as_ref()
                        .expect("default on required column with default"),
                    &columns.next.column.tpe.family
                )
            )
        }))
        .peekable();

    while let Some(source_column) = source_columns.next() {
        write!(query, "{}", source_column)?;

        if source_columns.peek().is_some() {
            write!(query, ", ")?;
        }
    }

    write!(
        query,
        " FROM {}.{}",
        Quoted::sqlite_ident(schema_name),
        Quoted::sqlite_ident(&differ.previous.name())
    )?;

    steps.push(SqlMigrationStep::RawSql { raw: query });

    Ok(())
}

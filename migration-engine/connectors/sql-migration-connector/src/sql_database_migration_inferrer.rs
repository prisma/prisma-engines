use crate::sql_schema_calculator::SqlSchemaCalculator;
use crate::sql_schema_differ::{SqlSchemaDiff, SqlSchemaDiffer};
use crate::*;
use datamodel::*;
use migration_connector::steps::MigrationStep;
use migration_connector::*;
use sql_schema_describer::*;
use std::sync::Arc;

pub struct SqlDatabaseMigrationInferrer {
    pub connection_info: ConnectionInfo,
    pub describer: Arc<dyn SqlSchemaDescriberBackend + Send + Sync + 'static>,
    pub schema_name: String,
}

#[async_trait::async_trait]
impl DatabaseMigrationInferrer<SqlMigration> for SqlDatabaseMigrationInferrer {
    async fn infer(
        &self,
        _previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let fut = async {
            let current_database_schema: SqlSchema = self.describe(&self.schema_name).await?;
            let expected_database_schema = SqlSchemaCalculator::calculate(next)?;
            infer(
                &current_database_schema,
                &expected_database_schema,
                &self.schema_name,
                self.sql_family(),
            )
        };

        catch(&self.connection_info, fut).await
    }

    async fn infer_from_datamodels(
        &self,
        previous: &Datamodel,
        next: &Datamodel,
        _steps: &[MigrationStep],
    ) -> ConnectorResult<SqlMigration> {
        let result: SqlResult<SqlMigration> = (|| {
            let current_database_schema: SqlSchema = SqlSchemaCalculator::calculate(previous)?;
            let expected_database_schema = SqlSchemaCalculator::calculate(next)?;
            infer(
                &current_database_schema,
                &expected_database_schema,
                &self.schema_name,
                self.sql_family(),
            )
        })();

        result.map_err(|sql_error| sql_error.into_connector_error(&self.connection_info))
    }
}

impl SqlDatabaseMigrationInferrer {
    async fn describe(&self, schema: &str) -> SqlResult<SqlSchema> {
        Ok(self.describer.describe(&schema).await?)
    }

    fn sql_family(&self) -> SqlFamily {
        self.connection_info.sql_family()
    }
}

fn infer(
    current_database_schema: &SqlSchema,
    expected_database_schema: &SqlSchema,
    schema_name: &str,
    sql_family: SqlFamily,
) -> SqlResult<SqlMigration> {
    let (original_steps, corrected_steps) = infer_database_migration_steps_and_fix(
        &current_database_schema,
        &expected_database_schema,
        &schema_name,
        sql_family,
    )?;
    let (_, rollback) = infer_database_migration_steps_and_fix(
        &expected_database_schema,
        &current_database_schema,
        &schema_name,
        sql_family,
    )?;
    Ok(SqlMigration {
        before: current_database_schema.clone(),
        after: expected_database_schema.clone(),
        original_steps,
        corrected_steps,
        rollback,
    })
}

fn infer_database_migration_steps_and_fix(
    from: &SqlSchema,
    to: &SqlSchema,
    schema_name: &str,
    sql_family: SqlFamily,
) -> SqlResult<(Vec<SqlMigrationStep>, Vec<SqlMigrationStep>)> {
    let diff: SqlSchemaDiff = SqlSchemaDiffer::diff(&from, &to, sql_family);
    let is_sqlite = sql_family == SqlFamily::Sqlite;

    let corrected_steps = if is_sqlite {
        fix_stupid_sqlite(diff, &from, &to, &schema_name)?
    } else {
        fix_id_column_type_change(&from, &to, schema_name, diff.into_steps(), sql_family)?
    };

    Ok((
        SqlSchemaDiffer::diff(&from, &to, sql_family).into_steps(),
        corrected_steps,
    ))
}

fn fix_id_column_type_change(
    from: &SqlSchema,
    to: &SqlSchema,
    _schema_name: &str,
    steps: Vec<SqlMigrationStep>,
    sql_family: SqlFamily,
) -> SqlResult<Vec<SqlMigrationStep>> {
    let has_id_type_change = steps
        .iter()
        .find(|step| match step {
            SqlMigrationStep::AlterTable(alter_table) => {
                if let Ok(current_table) = from.table(&alter_table.table.name) {
                    let change_to_id_column = alter_table.changes.iter().find(|c| match c {
                        TableChange::AlterColumn(alter_column) => {
                            let current_column = current_table.column_bang(&alter_column.name);
                            let current_column_type = &current_column.tpe;
                            let has_type_changed = current_column_type.family != alter_column.column.tpe.family; // TODO: take into account raw type
                            let is_part_of_pk = current_table
                                .primary_key
                                .clone()
                                .map(|pk| pk.columns)
                                .unwrap_or(vec![])
                                .contains(&alter_column.name);
                            is_part_of_pk && has_type_changed
                        }
                        _ => false,
                    });
                    change_to_id_column.is_some()
                } else {
                    false
                }
            }
            _ => false,
        })
        .is_some();

    // TODO: There's probably a much more graceful way to handle this. But this would also involve a lot of data loss probably. Let's tackle that after P Day
    if has_id_type_change {
        let mut radical_steps = Vec::new();
        let tables_to_drop: Vec<String> = from
            .tables
            .iter()
            .filter(|t| t.name != "_Migration")
            .map(|t| t.name.clone())
            .collect();
        radical_steps.push(SqlMigrationStep::DropTables(DropTables { names: tables_to_drop }));
        let diff_from_empty: SqlSchemaDiff = SqlSchemaDiffer::diff(&SqlSchema::empty(), &to, sql_family);
        let mut steps_from_empty = diff_from_empty.into_steps();
        radical_steps.append(&mut steps_from_empty);

        Ok(radical_steps)
    } else {
        Ok(steps)
    }
}

fn fix_stupid_sqlite(
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
                    table: next.name.clone(),
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

pub fn wrap_as_step<T, F>(steps: Vec<T>, mut wrap_fn: F) -> impl Iterator<Item = SqlMigrationStep>
where
    F: FnMut(T) -> SqlMigrationStep,
{
    steps.into_iter().map(move |x| wrap_fn(x))
}

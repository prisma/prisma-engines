use crate::*;
use migration_connector::*;
use sql_renderer::SqlRenderer;
use sql_schema_describer::*;
use std::sync::Arc;

pub struct SqlDatabaseStepApplier {
    pub sql_family: SqlFamily,
    pub schema_name: String,
    pub conn: Arc<dyn MigrationDatabase + Send + Sync + 'static>,
}

#[allow(unused, dead_code)]
impl DatabaseMigrationStepApplier<SqlMigration> for SqlDatabaseStepApplier {
    fn apply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        Ok(self.apply_next_step(&database_migration.corrected_steps, index)?)
    }

    fn unapply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        Ok(self.apply_next_step(&database_migration.rollback, index)?)
    }

    fn render_steps_pretty(&self, database_migration: &SqlMigration) -> ConnectorResult<serde_json::Value> {
        Ok(render_steps_pretty(
            &database_migration,
            self.sql_family,
            &self.schema_name,
        )?)
    }
}

impl SqlDatabaseStepApplier {
    fn apply_next_step(&self, steps: &Vec<SqlMigrationStep>, index: usize) -> SqlResult<bool> {
        let has_this_one = steps.get(index).is_some();
        if !has_this_one {
            return Ok(false);
        }

        let step = &steps[index];
        let sql_string = render_raw_sql(&step, self.sql_family, &self.schema_name);
        debug!("{}", sql_string);

        let result = self.conn.query_raw(&self.schema_name, &sql_string, &[]);

        // TODO: this does not evaluate the results of SQLites PRAGMA foreign_key_check
        result?;

        let has_more = steps.get(index + 1).is_some();
        Ok(has_more)
    }
}

fn render_steps_pretty(
    database_migration: &SqlMigration,
    sql_family: SqlFamily,
    schema_name: &str,
) -> ConnectorResult<serde_json::Value> {
    let jsons = database_migration
        .corrected_steps
        .iter()
        .map(|step| {
            let cloned = step.clone();
            let mut json_value = serde_json::to_value(&step).unwrap();
            let json_object = json_value.as_object_mut().unwrap();
            json_object.insert(
                "raw".to_string(),
                serde_json::Value::String(render_raw_sql(&cloned, sql_family, schema_name)),
            );
            json_value
        })
        .collect();
    Ok(serde_json::Value::Array(jsons))
}

fn render_raw_sql(step: &SqlMigrationStep, sql_family: SqlFamily, schema_name: &str) -> String {
    let schema_name = schema_name.to_string();
    let renderer = SqlRenderer::for_family(&sql_family);

    match step {
        SqlMigrationStep::CreateTable(CreateTable { table }) => {
            let cloned_columns = table.columns.clone();
            let primary_columns = table.primary_key_columns();
            let mut lines = Vec::new();
            for column in cloned_columns.clone() {
                let col_sql = renderer.render_column(&schema_name, &table, &column, false);
                lines.push(format!("  {}", col_sql));
            }
            let primary_key_was_already_set_in_column_line = lines.join(",").contains(&"PRIMARY KEY");

            if primary_columns.len() > 0 && !primary_key_was_already_set_in_column_line {
                let column_names: Vec<String> = primary_columns
                    .clone()
                    .into_iter()
                    .map(|col| renderer.quote(&col))
                    .collect();
                lines.push(format!("  PRIMARY KEY ({})", column_names.join(",")))
            }
            format!(
                "CREATE TABLE {} (\n{}\n){};",
                renderer.quote_with_schema(&schema_name, &table.name),
                lines.join(",\n"),
                create_table_suffix(sql_family),
            )
        }
        SqlMigrationStep::DropTable(DropTable { name }) => {
            format!("DROP TABLE {};", renderer.quote_with_schema(&schema_name, &name))
        }
        SqlMigrationStep::DropTables(DropTables { names }) => {
            let fully_qualified_names: Vec<String> = names
                .iter()
                .map(|name| renderer.quote_with_schema(&schema_name, &name))
                .collect();
            format!("DROP TABLE {};", fully_qualified_names.join(","))
        }
        SqlMigrationStep::RenameTable { name, new_name } => {
            let new_name = match sql_family {
                SqlFamily::Sqlite => renderer.quote(new_name),
                _ => renderer.quote_with_schema(&schema_name, &new_name),
            };
            format!(
                "ALTER TABLE {} RENAME TO {};",
                renderer.quote_with_schema(&schema_name, &name),
                new_name
            )
        }
        SqlMigrationStep::AlterTable(AlterTable { table, changes }) => {
            let mut lines = Vec::new();
            for change in changes.clone() {
                match change {
                    TableChange::AddColumn(AddColumn { column }) => {
                        let col_sql = renderer.render_column(&schema_name, &table, &column, true);
                        lines.push(format!("ADD COLUMN {}", col_sql));
                    }
                    TableChange::DropColumn(DropColumn { name }) => {
                        // TODO: this does not work on MySQL for columns with foreign keys. Here the FK must be dropped first by name.
                        let name = renderer.quote(&name);
                        lines.push(format!("DROP COLUMN {}", name));
                    }
                    TableChange::AlterColumn(AlterColumn { name, column }) => {
                        let name = renderer.quote(&name);
                        lines.push(format!("DROP COLUMN {}", name));
                        let col_sql = renderer.render_column(&schema_name, &table, &column, true);
                        lines.push(format!("ADD COLUMN {}", col_sql));
                    }
                }
            }
            format!(
                "ALTER TABLE {} {};",
                renderer.quote_with_schema(&schema_name, &table.name),
                lines.join(",\n")
            )
        }
        SqlMigrationStep::CreateIndex(CreateIndex { table, index }) => {
            let Index { name, columns, tpe } = index;
            let index_type = match tpe {
                IndexType::Unique => "UNIQUE",
                IndexType::Normal => "",
            };
            let index_name = match sql_family {
                SqlFamily::Sqlite => renderer.quote_with_schema(&schema_name, &name),
                _ => renderer.quote(&name),
            };
            let table_reference = match sql_family {
                SqlFamily::Sqlite => renderer.quote(&table),
                _ => renderer.quote_with_schema(&schema_name, &table),
            };
            let columns: Vec<String> = columns.iter().map(|c| renderer.quote(c)).collect();
            format!(
                "CREATE {} INDEX {} ON {}({})",
                index_type,
                index_name,
                table_reference,
                columns.join(",")
            )
        }
        SqlMigrationStep::DropIndex(DropIndex { table, name }) => match sql_family {
            SqlFamily::Mysql => format!(
                "DROP INDEX {} ON {}",
                renderer.quote(&name),
                renderer.quote_with_schema(&schema_name, &table),
            ),
            SqlFamily::Postgres | SqlFamily::Sqlite => {
                format!("DROP INDEX {}", renderer.quote_with_schema(&schema_name, &name),)
            }
        },
        SqlMigrationStep::RawSql { raw } => raw.to_string(),
    }
}

fn create_table_suffix(sql_family: SqlFamily) -> &'static str {
    match sql_family {
        SqlFamily::Sqlite => "",
        SqlFamily::Postgres => "",
        SqlFamily::Mysql => "\nDEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
    }
}

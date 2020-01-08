use crate::*;
use quaint::prelude::Queryable;
use sql_renderer::SqlRenderer;
use sql_schema_describer::*;
use std::sync::Arc;
use tracing_futures::Instrument;

pub struct SqlDatabaseStepApplier {
    pub connection_info: ConnectionInfo,
    pub schema_name: String,
    pub conn: Arc<dyn Queryable + Send + Sync + 'static>,
}

#[async_trait::async_trait]
impl DatabaseMigrationStepApplier<SqlMigration> for SqlDatabaseStepApplier {
    async fn apply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        self.catch(
            self.apply_next_step(&database_migration.corrected_steps, index, self.renderer().as_ref())
                .instrument(tracing::debug_span!("ApplySqlStep", index)),
        )
        .await
    }

    async fn unapply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        self.catch(
            self.apply_next_step(&database_migration.rollback, index, self.renderer().as_ref())
                .instrument(tracing::debug_span!("UnapplySqlStep", index)),
        )
        .await
    }

    fn render_steps_pretty(&self, database_migration: &SqlMigration) -> ConnectorResult<Vec<serde_json::Value>> {
        Ok(
            render_steps_pretty(&database_migration, self.renderer().as_ref(), &self.schema_name)?
                .into_iter()
                .map(|pretty_step| serde_json::to_value(&pretty_step).unwrap())
                .collect(),
        )
    }
}

impl SqlDatabaseStepApplier {
    async fn catch<O>(&self, fut: impl std::future::Future<Output = SqlResult<O>>) -> ConnectorResult<O> {
        match fut.await {
            Ok(o) => Ok(o),
            Err(sql_error) => Err(sql_error.into_connector_error(&self.connection_info)),
        }
    }

    async fn apply_next_step(
        &self,
        steps: &[SqlMigrationStep],
        index: usize,
        renderer: &(dyn SqlRenderer + Send + Sync),
    ) -> SqlResult<bool> {
        let has_this_one = steps.get(index).is_some();
        if !has_this_one {
            return Ok(false);
        }

        let step = &steps[index];
        tracing::debug!(?step);

        if let Some(sql_string) = render_raw_sql(&step, renderer, &self.schema_name)
            .map_err(|err: std::fmt::Error| SqlError::Generic(format!("IO error: {}", err)))?
        {
            tracing::debug!(index, %sql_string);

            let result = self.conn.query_raw(&sql_string, &[]).await;

            // TODO: this does not evaluate the results of SQLites PRAGMA foreign_key_check
            result?;
        } else {
            tracing::debug!("Step rendered no SQL.");
        }

        let has_more = steps.get(index + 1).is_some();
        Ok(has_more)
    }

    fn sql_family(&self) -> SqlFamily {
        self.connection_info.sql_family()
    }

    fn renderer<'a>(&'a self) -> Box<dyn SqlRenderer + Send + Sync + 'a> {
        SqlRenderer::for_family(&self.sql_family())
    }
}

fn render_steps_pretty(
    database_migration: &SqlMigration,
    renderer: &(dyn SqlRenderer + Send + Sync),
    schema_name: &str,
) -> ConnectorResult<Vec<PrettySqlMigrationStep>> {
    let mut steps = Vec::with_capacity(database_migration.corrected_steps.len());

    for step in &database_migration.corrected_steps {
        if let Some(sql) = render_raw_sql(&step, renderer, schema_name)
            .map_err(|err: std::fmt::Error| ConnectorError::from_kind(ErrorKind::Generic(err.into())))?
        {
            steps.push(PrettySqlMigrationStep {
                step: step.clone(),
                raw: sql,
            });
        }
    }

    Ok(steps)
}

fn render_raw_sql(
    step: &SqlMigrationStep,
    renderer: &(dyn SqlRenderer + Send + Sync),
    schema_name: &str,
) -> std::result::Result<Option<String>, std::fmt::Error> {
    use itertools::Itertools;
    use std::fmt::Write as _;

    let sql_family = renderer.sql_family();
    let schema_name = schema_name.to_string();

    match step {
        SqlMigrationStep::CreateTable(CreateTable { table }) => {
            let mut create_table = String::with_capacity(100);

            write!(create_table, "CREATE TABLE ")?;
            renderer.write_quoted_with_schema(&mut create_table, &schema_name, &table.name)?;
            writeln!(create_table, " (")?;

            let mut columns = table.columns.iter().peekable();
            while let Some(column) = columns.next() {
                let col_sql = renderer.render_column(&schema_name, &table, &column, false);

                write!(
                    create_table,
                    "    {}{}",
                    col_sql,
                    if columns.peek().is_some() { ",\n" } else { "" }
                )?;
            }

            let primary_key_is_already_set = create_table.contains("PRIMARY KEY");
            let primary_columns = table.primary_key_columns();

            if primary_columns.len() > 0 && !primary_key_is_already_set {
                let column_names: Vec<String> = primary_columns
                    .clone()
                    .into_iter()
                    .map(|col| renderer.quote(&col))
                    .collect();
                write!(create_table, ",\n    PRIMARY KEY ({})", column_names.join(","))?;
            }

            if sql_family == SqlFamily::Sqlite && !table.foreign_keys.is_empty() {
                write!(create_table, ",")?;

                let mut fks = table.foreign_keys.iter().peekable();

                while let Some(fk) = fks.next() {
                    write!(
                        create_table,
                        "FOREIGN KEY ({constrained_columns}) {references}{comma}",
                        constrained_columns = fk.columns.iter().map(|col| format!(r#""{}""#, col)).join(","),
                        references = renderer.render_references(&schema_name, fk),
                        comma = if fks.peek().is_some() { ",\n" } else { "" },
                    )?;
                }
            }

            write!(create_table, "\n) {}", create_table_suffix(sql_family))?;

            Ok(Some(create_table))
        }
        SqlMigrationStep::DropTable(DropTable { name }) => Ok(Some(format!(
            "DROP TABLE {};",
            renderer.quote_with_schema(&schema_name, &name)
        ))),
        SqlMigrationStep::DropTables(DropTables { names }) => {
            let fully_qualified_names: Vec<String> = names
                .iter()
                .map(|name| renderer.quote_with_schema(&schema_name, &name))
                .collect();
            Ok(Some(format!("DROP TABLE {};", fully_qualified_names.join(","))))
        }
        SqlMigrationStep::RenameTable { name, new_name } => {
            let new_name = match sql_family {
                SqlFamily::Sqlite => renderer.quote(new_name),
                _ => renderer.quote_with_schema(&schema_name, &new_name),
            };
            Ok(Some(format!(
                "ALTER TABLE {} RENAME TO {};",
                renderer.quote_with_schema(&schema_name, &name),
                new_name
            )))
        }
        SqlMigrationStep::AddForeignKey(AddForeignKey { table, foreign_key }) => match sql_family {
            SqlFamily::Sqlite => Ok(None),
            _ => {
                let mut add_constraint = String::with_capacity(120);

                write!(
                    add_constraint,
                    "ALTER TABLE {table} ADD ",
                    table = renderer.quote_with_schema(&schema_name, table)
                )?;

                if let Some(constraint_name) = foreign_key.constraint_name.as_ref() {
                    write!(add_constraint, "CONSTRAINT {} ", renderer.quote(constraint_name))?;
                }

                write!(add_constraint, "FOREIGN KEY (")?;

                for (idx, column) in foreign_key.columns.iter().enumerate() {
                    write!(add_constraint, "{}", renderer.quote(column))?;

                    if idx < foreign_key.columns.len() - 1 {
                        write!(add_constraint, ", ")?;
                    }
                }

                write!(add_constraint, ") ")?;

                add_constraint.push_str(&renderer.render_references(&schema_name, &foreign_key));

                Ok(Some(add_constraint))
            }
        },
        SqlMigrationStep::AlterTable(AlterTable { table, changes }) => {
            let mut lines = Vec::new();
            for change in changes.clone() {
                match change {
                    TableChange::AddColumn(AddColumn { column }) => {
                        let col_sql = renderer.render_column(&schema_name, &table, &column, true);
                        lines.push(format!("ADD COLUMN {}", col_sql));
                    }
                    TableChange::DropColumn(DropColumn { name }) => {
                        let name = renderer.quote(&name);
                        lines.push(format!("DROP COLUMN {}", name));
                    }
                    TableChange::AlterColumn(AlterColumn { name, column }) => {
                        let name = renderer.quote(&name);
                        lines.push(format!("DROP COLUMN {}", name));
                        let col_sql = renderer.render_column(&schema_name, &table, &column, true);
                        lines.push(format!("ADD COLUMN {}", col_sql));
                    }
                    TableChange::DropForeignKey(DropForeignKey { constraint_name }) => match sql_family {
                        SqlFamily::Mysql => {
                            let constraint_name = renderer.quote(&constraint_name);
                            lines.push(format!("DROP FOREIGN KEY {}", constraint_name));
                        }
                        _ => (),
                    },
                };
            }
            Ok(Some(format!(
                "ALTER TABLE {} {};",
                renderer.quote_with_schema(&schema_name, &table.name),
                lines.join(",\n")
            )))
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
            Ok(Some(format!(
                "CREATE {} INDEX {} ON {}({})",
                index_type,
                index_name,
                table_reference,
                columns.join(",")
            )))
        }
        SqlMigrationStep::DropIndex(DropIndex { table, name }) => match sql_family {
            SqlFamily::Mysql => Ok(Some(format!(
                "DROP INDEX {} ON {}",
                renderer.quote(&name),
                renderer.quote_with_schema(&schema_name, &table),
            ))),
            SqlFamily::Postgres | SqlFamily::Sqlite => Ok(Some(format!(
                "DROP INDEX {}",
                renderer.quote_with_schema(&schema_name, &name)
            ))),
        },
        SqlMigrationStep::AlterIndex(AlterIndex {
            table,
            index_name,
            index_new_name,
        }) => match sql_family {
            SqlFamily::Mysql => Ok(Some(format!(
                "ALTER TABLE {table_name} RENAME INDEX {index_name} TO {index_new_name}",
                table_name = renderer.quote_with_schema(&schema_name, &table),
                index_name = renderer.quote(index_name),
                index_new_name = renderer.quote(index_new_name)
            ))),
            SqlFamily::Postgres => Ok(Some(format!(
                "ALTER INDEX {} RENAME TO {}",
                renderer.quote_with_schema(&schema_name, index_name),
                renderer.quote(index_new_name)
            ))),
            SqlFamily::Sqlite => unimplemented!("Index renaming on SQLite."),
        },
        SqlMigrationStep::RawSql { raw } => Ok(Some(raw.to_owned())),
    }
}

fn create_table_suffix(sql_family: SqlFamily) -> &'static str {
    match sql_family {
        SqlFamily::Sqlite => "",
        SqlFamily::Postgres => "",
        SqlFamily::Mysql => "\nDEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
    }
}

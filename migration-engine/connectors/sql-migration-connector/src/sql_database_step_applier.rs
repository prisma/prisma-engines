use crate::*;
use sql_renderer::{postgres_render_column_type, rendered_step::RenderedStep, IteratorJoin, Quoted, SqlRenderer};
use sql_schema_describer::*;
use sql_schema_differ::DiffingOptions;
use sql_schema_helpers::{walk_columns, ColumnRef};
use std::fmt::Write as _;
use tracing_futures::Instrument;

pub struct SqlDatabaseStepApplier<'a> {
    pub connector: &'a crate::SqlMigrationConnector,
}

impl crate::component::Component for SqlDatabaseStepApplier<'_> {
    fn connector(&self) -> &crate::SqlMigrationConnector {
        self.connector
    }
}

#[async_trait::async_trait]
impl DatabaseMigrationStepApplier<SqlMigration> for SqlDatabaseStepApplier<'_> {
    async fn apply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        let renderer = self.renderer();
        let fut = self
            .apply_next_step(
                &database_migration.corrected_steps,
                index,
                renderer.as_ref(),
                &database_migration.before,
                &database_migration.after,
            )
            .instrument(tracing::debug_span!("ApplySqlStep", index));

        crate::catch(self.connection_info(), fut).await
    }

    async fn unapply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        let renderer = self.renderer();
        let fut = self
            .apply_next_step(
                &database_migration.rollback,
                index,
                renderer.as_ref(),
                &database_migration.after,
                &database_migration.before,
            )
            .instrument(tracing::debug_span!("UnapplySqlStep", index));

        crate::catch(self.connection_info(), fut).await
    }

    fn render_steps_pretty(
        &self,
        database_migration: &SqlMigration,
    ) -> ConnectorResult<Vec<PrettyDatabaseMigrationStep>> {
        render_steps_pretty(
            &database_migration,
            self.renderer().as_ref(),
            self.database_info(),
            &database_migration.before,
            &database_migration.after,
        )
    }
}

impl SqlDatabaseStepApplier<'_> {
    async fn apply_next_step(
        &self,
        steps: &[SqlMigrationStep],
        index: usize,
        renderer: &(dyn SqlRenderer + Send + Sync),
        current_schema: &SqlSchema,
        next_schema: &SqlSchema,
    ) -> SqlResult<bool> {
        let has_this_one = steps.get(index).is_some();
        if !has_this_one {
            return Ok(false);
        }

        let step = &steps[index];
        tracing::debug!(?step);

        for sql_string in render_raw_sql(&step, renderer, self.database_info(), current_schema, next_schema)
            .map_err(|err: anyhow::Error| SqlError::Generic(err))?
        {
            tracing::debug!(index, %sql_string);

            let result = self.conn().query_raw(&sql_string, &[]).await;

            // TODO: this does not evaluate the results of SQLites PRAGMA foreign_key_check
            result?;
        }

        let has_more = steps.get(index + 1).is_some();
        Ok(has_more)
    }

    fn renderer<'a>(&'a self) -> Box<dyn SqlRenderer + Send + Sync + 'a> {
        SqlRenderer::for_family(&self.sql_family())
    }
}

fn render_steps_pretty(
    database_migration: &SqlMigration,
    renderer: &(dyn SqlRenderer + Send + Sync),
    database_info: &DatabaseInfo,
    current_schema: &SqlSchema,
    next_schema: &SqlSchema,
) -> ConnectorResult<Vec<PrettyDatabaseMigrationStep>> {
    let mut steps = Vec::with_capacity(database_migration.corrected_steps.len());

    for step in &database_migration.corrected_steps {
        let sql = render_raw_sql(&step, renderer, database_info, current_schema, next_schema)
            .map_err(|err: anyhow::Error| {
                ConnectorError::from_kind(migration_connector::ErrorKind::Generic(err.into()))
            })?
            .join(";\n");

        if !sql.is_empty() {
            steps.push(PrettyDatabaseMigrationStep {
                step: serde_json::to_value(&step).unwrap_or_else(|_| serde_json::json!({})),
                raw: sql,
            });
        }
    }

    Ok(steps)
}

fn render_raw_sql(
    step: &SqlMigrationStep,
    renderer: &(dyn SqlRenderer + Send + Sync),
    database_info: &DatabaseInfo,
    current_schema: &SqlSchema,
    next_schema: &SqlSchema,
) -> Result<Vec<String>, anyhow::Error> {
    let sql_family = renderer.sql_family();
    let schema_name = database_info.connection_info().schema_name().to_string();

    match step {
        SqlMigrationStep::CreateEnum(create_enum) => render_create_enum(renderer, create_enum),
        SqlMigrationStep::DropEnum(drop_enum) => render_drop_enum(renderer, drop_enum),
        SqlMigrationStep::AlterEnum(alter_enum) => match renderer.sql_family() {
            SqlFamily::Postgres => postgres_alter_enum(alter_enum, next_schema, &schema_name)?.into(),
            SqlFamily::Mysql => mysql_alter_enum(alter_enum, next_schema, &schema_name),
            _ => Ok(Vec::new()),
        },
        SqlMigrationStep::CreateTable(CreateTable { table }) => {
            let columns: String = table
                .columns
                .iter()
                .map(|column| {
                    let column = ColumnRef {
                        schema: next_schema,
                        column,
                        table,
                    };
                    renderer.render_column(&schema_name, column, false)
                })
                .join(",");

            let mut create_table = format!(
                "CREATE TABLE {} (\n{}",
                renderer.quote_with_schema(&schema_name, &table.name),
                columns,
            );

            let primary_key_is_already_set = create_table.contains("PRIMARY KEY");
            let primary_columns = table.primary_key_columns();

            if primary_columns.len() > 0 && !primary_key_is_already_set {
                let column_names = primary_columns.iter().map(|col| renderer.quote(&col)).join(",");
                write!(create_table, ",\n    PRIMARY KEY ({})", column_names)?;
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

            create_table.push_str(create_table_suffix(sql_family));

            Ok(vec![create_table])
        }
        SqlMigrationStep::DropTable(DropTable { name }) => match sql_family {
            SqlFamily::Mysql | SqlFamily::Postgres => Ok(vec![format!(
                "DROP TABLE {};",
                renderer.quote_with_schema(&schema_name, &name)
            )]),
            // Turning off the pragma is safe, because schema validation would forbid foreign keys
            // to a non-existent model. There appears to be no other way to deal with cyclic
            // dependencies in the dropping order of tables in the presence of foreign key
            // constraints on SQLite.
            SqlFamily::Sqlite => Ok(vec![
                "PRAGMA foreign_keys=off".to_string(),
                format!("DROP TABLE {};", renderer.quote_with_schema(&schema_name, &name)),
                "PRAGMA foreign_keys=on".to_string(),
            ]),
            SqlFamily::Mssql => todo!("Greetings from Redmond"),
        },
        SqlMigrationStep::RenameTable { name, new_name } => {
            let new_name = match sql_family {
                SqlFamily::Sqlite => renderer.quote(new_name).to_string(),
                _ => renderer.quote_with_schema(&schema_name, &new_name).to_string(),
            };
            Ok(vec![format!(
                "ALTER TABLE {} RENAME TO {};",
                renderer.quote_with_schema(&schema_name, &name),
                new_name
            )])
        }
        SqlMigrationStep::AddForeignKey(AddForeignKey { table, foreign_key }) => match sql_family {
            SqlFamily::Sqlite => Ok(Vec::new()),
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

                write!(
                    add_constraint,
                    "FOREIGN KEY ({})",
                    foreign_key.columns.iter().map(|col| renderer.quote(col)).join(", ")
                )?;

                add_constraint.push_str(&renderer.render_references(&schema_name, &foreign_key));

                Ok(vec![add_constraint])
            }
        },
        SqlMigrationStep::AlterTable(AlterTable { table, changes }) => {
            let mut lines = Vec::new();
            for change in changes {
                match change {
                    TableChange::AddColumn(AddColumn { column }) => {
                        let column = ColumnRef {
                            table,
                            schema: next_schema,
                            column,
                        };
                        let col_sql = renderer.render_column(&schema_name, column, true);
                        lines.push(format!("ADD COLUMN {}", col_sql));
                    }
                    TableChange::DropColumn(DropColumn { name }) => {
                        let name = renderer.quote(&name);
                        lines.push(format!("DROP COLUMN {}", name));
                    }
                    TableChange::AlterColumn(AlterColumn { name, column }) => {
                        match safe_alter_column(
                            renderer,
                            current_schema.get_table(&table.name).unwrap().column(&name).unwrap(),
                            &column,
                            &DiffingOptions::from_database_info(database_info),
                        ) {
                            Some(safe_sql) => {
                                for line in safe_sql {
                                    lines.push(line)
                                }
                            }
                            None => {
                                let name = renderer.quote(&name);
                                lines.push(format!("DROP COLUMN {}", name));
                                let column = ColumnRef {
                                    schema: next_schema,
                                    table,
                                    column,
                                };
                                let col_sql = renderer.render_column(&schema_name, column, true);
                                lines.push(format!("ADD COLUMN {}", col_sql));
                            }
                        }
                    }
                    TableChange::DropForeignKey(DropForeignKey { constraint_name }) => match sql_family {
                        SqlFamily::Mysql => {
                            let constraint_name = renderer.quote(&constraint_name);
                            lines.push(format!("DROP FOREIGN KEY {}", constraint_name));
                        }
                        SqlFamily::Postgres => {
                            let constraint_name = renderer.quote(&constraint_name);
                            lines.push(format!("DROP CONSTRAINT IF EXiSTS {}", constraint_name));
                        }
                        SqlFamily::Sqlite => (),
                        SqlFamily::Mssql => todo!("Greetings from Redmond"),
                    },
                };
            }

            if lines.is_empty() {
                return Ok(Vec::new());
            }

            Ok(vec![format!(
                "ALTER TABLE {} {};",
                renderer.quote_with_schema(&schema_name, &table.name),
                lines.join(",\n")
            )])
        }
        SqlMigrationStep::CreateIndex(CreateIndex { table, index }) => {
            Ok(vec![render_create_index(renderer, database_info, table, index)])
        }
        SqlMigrationStep::DropIndex(DropIndex { table, name }) => match sql_family {
            SqlFamily::Mysql => Ok(vec![format!(
                "DROP INDEX {} ON {}",
                renderer.quote(&name),
                renderer.quote_with_schema(&schema_name, &table),
            )]),
            SqlFamily::Postgres | SqlFamily::Sqlite => Ok(vec![format!(
                "DROP INDEX {}",
                renderer.quote_with_schema(&schema_name, &name)
            )]),
            SqlFamily::Mssql => todo!("Greetings from Redmond"),
        },
        SqlMigrationStep::AlterIndex(AlterIndex {
            table,
            index_name,
            index_new_name,
        }) => match sql_family {
            SqlFamily::Mssql => todo!("Greetings from Redmond"),
            SqlFamily::Mysql => {
                // MariaDB and MySQL 5.6 do not support `ALTER TABLE ... RENAME INDEX`.
                if database_info.is_mariadb() || database_info.is_mysql_5_6() {
                    let old_index = current_schema
                        .table(table)
                        .map_err(|_| {
                            anyhow::anyhow!(
                                "Invariant violation: could not find table `{}` in current schema.",
                                table
                            )
                        })?
                        .indices
                        .iter()
                        .find(|idx| idx.name.as_str() == index_name)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Invariant violation: could not find index `{}` on table `{}` in current schema.",
                                index_name,
                                table
                            )
                        })?;
                    let mut new_index = old_index.clone();
                    new_index.name = index_new_name.clone();

                    // Order matters: dropping the old index first wouldn't work when foreign key constraints are still relying on it.
                    Ok(vec![
                        render_create_index(renderer, database_info, table, &new_index),
                        mysql_drop_index(renderer, &schema_name, table, index_name)?,
                    ])
                } else {
                    Ok(vec![format!(
                        "ALTER TABLE {table_name} RENAME INDEX {index_name} TO {index_new_name}",
                        table_name = renderer.quote_with_schema(&schema_name, &table),
                        index_name = renderer.quote(index_name),
                        index_new_name = renderer.quote(index_new_name)
                    )])
                }
            }
            SqlFamily::Postgres => Ok(vec![format!(
                "ALTER INDEX {} RENAME TO {}",
                renderer.quote_with_schema(&schema_name, index_name),
                renderer.quote(index_new_name)
            )]),
            SqlFamily::Sqlite => unimplemented!("Index renaming on SQLite."),
        },
        SqlMigrationStep::RawSql { raw } => Ok(vec![raw.to_owned()]),
    }
}

fn render_create_index(
    renderer: &dyn SqlRenderer,
    database_info: &DatabaseInfo,
    table_name: &str,
    index: &Index,
) -> String {
    let Index { name, columns, tpe } = index;
    let index_type = match tpe {
        IndexType::Unique => "UNIQUE",
        IndexType::Normal => "",
    };
    let sql_family = database_info.sql_family();
    let index_name = match sql_family {
        SqlFamily::Sqlite => renderer
            .quote_with_schema(database_info.connection_info().schema_name(), &name)
            .to_string(),
        _ => renderer.quote(&name).to_string(),
    };
    let table_reference = match sql_family {
        SqlFamily::Sqlite => renderer.quote(table_name).to_string(),
        _ => renderer
            .quote_with_schema(database_info.connection_info().schema_name(), table_name)
            .to_string(),
    };
    let columns = columns.iter().map(|c| renderer.quote(c));

    format!(
        "CREATE {} INDEX {} ON {}({})",
        index_type,
        index_name,
        table_reference,
        columns.join(",")
    )
}

fn mysql_drop_index(
    renderer: &dyn SqlRenderer,
    schema_name: &str,
    table_name: &str,
    index_name: &str,
) -> Result<String, std::fmt::Error> {
    Ok(format!(
        "DROP INDEX {} ON {}",
        renderer.quote(index_name),
        renderer.quote_with_schema(schema_name, table_name)
    ))
}

fn create_table_suffix(sql_family: SqlFamily) -> &'static str {
    match sql_family {
        SqlFamily::Sqlite => ")",
        SqlFamily::Postgres => ")",
        SqlFamily::Mysql => "\n) DEFAULT CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci",
        SqlFamily::Mssql => todo!("Greetings from Redmond"),
    }
}

fn safe_alter_column(
    renderer: &dyn SqlRenderer,
    previous_column: &Column,
    next_column: &Column,
    diffing_options: &DiffingOptions,
) -> Option<Vec<String>> {
    use crate::sql_migration::expanded_alter_column::*;

    let expanded = crate::sql_migration::expanded_alter_column::expand_alter_column(
        previous_column,
        next_column,
        &renderer.sql_family(),
        diffing_options,
    )?;

    let alter_column_prefix = format!("ALTER COLUMN {}", renderer.quote(&previous_column.name));

    let steps = match expanded {
        ExpandedAlterColumn::Postgres(steps) => steps
            .into_iter()
            .map(|step| match step {
                PostgresAlterColumn::DropDefault => format!("{} DROP DEFAULT", &alter_column_prefix),
                PostgresAlterColumn::SetDefault(new_default) => format!(
                    "{} SET DEFAULT {}",
                    &alter_column_prefix,
                    renderer.render_default(&new_default, &next_column.tpe.family)
                ),
                PostgresAlterColumn::DropNotNull => format!("{} DROP NOT NULL", &alter_column_prefix),
                PostgresAlterColumn::SetType(ty) => format!(
                    "{} SET DATA TYPE {}",
                    &alter_column_prefix,
                    postgres_render_column_type(&ty)
                ),
            })
            .collect(),
        ExpandedAlterColumn::Mysql(steps) => steps
            .into_iter()
            .map(|step| match step {
                MysqlAlterColumn::DropDefault => format!("{} DROP DEFAULT", &alter_column_prefix),
                MysqlAlterColumn::SetDefault(new_default) => format!(
                    "{} SET DEFAULT {}",
                    &alter_column_prefix,
                    renderer.render_default(&new_default, &next_column.tpe.family)
                ),
            })
            .collect(),
        ExpandedAlterColumn::Sqlite(_steps) => vec![],
    };

    Some(steps)
}

fn render_create_enum(
    renderer: &(dyn SqlRenderer + Send + Sync),
    create_enum: &CreateEnum,
) -> Result<Vec<String>, anyhow::Error> {
    match renderer.sql_family() {
        SqlFamily::Postgres => {
            let sql = format!(
                r#"CREATE TYPE {enum_name} AS ENUM ({variants});"#,
                enum_name = Quoted::postgres_ident(&create_enum.name),
                variants = create_enum.variants.iter().map(Quoted::postgres_string).join(", "),
            );
            Ok(vec![sql])
        }
        _ => Ok(Vec::new()),
    }
}

fn render_drop_enum(
    renderer: &(dyn SqlRenderer + Send + Sync),
    drop_enum: &DropEnum,
) -> Result<Vec<String>, anyhow::Error> {
    match renderer.sql_family() {
        SqlFamily::Postgres => {
            let sql = format!(
                "DROP TYPE {enum_name}",
                enum_name = Quoted::postgres_ident(&drop_enum.name),
            );

            Ok(vec![sql])
        }
        _ => Ok(Vec::new()),
    }
}

fn postgres_alter_enum(
    alter_enum: &AlterEnum,
    next_schema: &SqlSchema,
    schema_name: &str,
) -> anyhow::Result<RenderedStep> {
    if alter_enum.dropped_variants.is_empty() {
        let stmts: Vec<String> = alter_enum
            .created_variants
            .iter()
            .map(|created_value| {
                format!(
                    "ALTER TYPE {enum_name} ADD VALUE {value}",
                    enum_name = Quoted::postgres_ident(&alter_enum.name),
                    value = Quoted::postgres_string(created_value)
                )
            })
            .collect();

        Ok(RenderedStep::new(stmts))
    } else {
        let new_enum = next_schema
            .get_enum(&alter_enum.name)
            .ok_or_else(|| anyhow::anyhow!("Enum `{}` not found in target schema.", alter_enum.name))?;

        let mut stmts = Vec::with_capacity(8);

        let tmp_name = format!("{}_new", &new_enum.name);
        let tmp_old_name = format!("{}_old", &alter_enum.name);

        // create the new enum with tmp name
        {
            let create_new_enum = format!(
                "CREATE TYPE {enum_name} AS ENUM ({variants})",
                enum_name = Quoted::postgres_ident(&tmp_name),
                variants = new_enum.values.iter().map(Quoted::postgres_string).join(", ")
            );

            stmts.push(create_new_enum);
        }

        // alter type of the current columns to new, with a cast
        {
            let affected_columns = walk_columns(next_schema).filter(|column| match &column.column_type().family {
                ColumnTypeFamily::Enum(name) if name.as_str() == alter_enum.name.as_str() => true,
                _ => false,
            });

            for column in affected_columns {
                let sql = format!(
                    "ALTER TABLE {schema_name}.{table_name} \
                        ALTER COLUMN {column_name} DROP DEFAULT,
                        ALTER COLUMN {column_name} TYPE {tmp_name} \
                            USING ({column_name}::text::{tmp_name}),
                        ALTER COLUMN {column_name} SET DEFAULT {new_enum_default}",
                    schema_name = Quoted::postgres_ident(schema_name),
                    table_name = Quoted::postgres_ident(column.table().name()),
                    column_name = Quoted::postgres_ident(column.name()),
                    tmp_name = Quoted::postgres_ident(&tmp_name),
                    new_enum_default = Quoted::postgres_string(new_enum.values.first().unwrap()),
                );

                stmts.push(sql);
            }
        }

        // rename old enum
        {
            let sql = format!(
                "ALTER TYPE {enum_name} RENAME TO {tmp_old_name}",
                enum_name = Quoted::postgres_ident(&alter_enum.name),
                tmp_old_name = Quoted::postgres_ident(&tmp_old_name)
            );

            stmts.push(sql);
        }

        // rename new enum
        {
            let sql = format!(
                "ALTER TYPE {tmp_name} RENAME TO {enum_name}",
                tmp_name = Quoted::postgres_ident(&tmp_name),
                enum_name = Quoted::postgres_ident(&new_enum.name)
            );

            stmts.push(sql)
        }

        // drop old enum
        {
            let sql = format!(
                "DROP TYPE {tmp_old_name}",
                tmp_old_name = Quoted::postgres_ident(&tmp_old_name),
            );

            stmts.push(sql)
        }

        Ok(RenderedStep::new(stmts).with_transaction(true))
    }
}

fn mysql_alter_enum(alter_enum: &AlterEnum, next_schema: &SqlSchema, schema_name: &str) -> anyhow::Result<Vec<String>> {
    let column = sql_schema_helpers::walk_columns(next_schema)
        .find(|col| match &col.column_type().family {
            ColumnTypeFamily::Enum(enum_name) if enum_name.as_str() == alter_enum.name.as_str() => true,
            _ => false,
        })
        .ok_or_else(|| anyhow::anyhow!("Could not find column to alter for {:?}", alter_enum))?;
    let enum_variants = next_schema
        .get_enum(&alter_enum.name)
        .ok_or_else(|| anyhow::anyhow!("Couldn't find enum {:?}", alter_enum.name))?
        .values
        .iter()
        .map(Quoted::mysql_string)
        .join(", ");

    let change_column = format!(
        "ALTER TABLE {schema_name}.{table_name} CHANGE {column_name} {column_name} ENUM({enum_variants})",
        schema_name = Quoted::mysql_ident(schema_name),
        table_name = Quoted::mysql_ident(column.table().name()),
        column_name = column.name(),
        enum_variants = enum_variants,
    );

    Ok(vec![change_column])
}

use crate::*;
use sql_migration::{CreateTable, DropForeignKey, DropTable, SqlMigrationStep};
use sql_renderer::{IteratorJoin, Quoted};
use sql_schema_describer::walkers::SqlSchemaExt;
use sql_schema_describer::{Index, IndexType, SqlSchema};
use sql_schema_differ::SqlSchemaDiffer;
use tracing_futures::Instrument;
use SqlFlavour;

pub struct SqlDatabaseStepApplier<'a> {
    pub connector: &'a crate::SqlMigrationConnector,
}

impl Component for SqlDatabaseStepApplier<'_> {
    fn connector(&self) -> &crate::SqlMigrationConnector {
        self.connector
    }
}

#[async_trait::async_trait]
impl DatabaseMigrationStepApplier<SqlMigration> for SqlDatabaseStepApplier<'_> {
    async fn apply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        let fut = self
            .apply_next_step(
                &database_migration.steps,
                index,
                self.flavour(),
                &database_migration.before,
                &database_migration.after,
            )
            .instrument(tracing::debug_span!("ApplySqlStep", index));

        crate::catch(self.connection_info(), fut).await
    }

    fn render_steps_pretty(
        &self,
        database_migration: &SqlMigration,
    ) -> ConnectorResult<Vec<PrettyDatabaseMigrationStep>> {
        render_steps_pretty(
            &database_migration,
            self.flavour(),
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
        renderer: &(dyn SqlFlavour + Send + Sync),
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
            .map_err(SqlError::Generic)?
        {
            tracing::debug!(index, %sql_string);

            self.conn().raw_cmd(&sql_string).await?;
        }

        Ok(true)
    }
}

fn render_steps_pretty(
    database_migration: &SqlMigration,
    renderer: &(dyn SqlFlavour + Send + Sync),
    database_info: &DatabaseInfo,
    current_schema: &SqlSchema,
    next_schema: &SqlSchema,
) -> ConnectorResult<Vec<PrettyDatabaseMigrationStep>> {
    let mut steps = Vec::with_capacity(database_migration.steps.len());

    for step in &database_migration.steps {
        let sql = render_raw_sql(&step, renderer, database_info, current_schema, next_schema)
            .map_err(|err: anyhow::Error| ConnectorError::from_kind(migration_connector::ErrorKind::Generic(err)))?
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
    renderer: &(dyn SqlFlavour + Send + Sync),
    database_info: &DatabaseInfo,
    current_schema: &SqlSchema,
    next_schema: &SqlSchema,
) -> Result<Vec<String>, anyhow::Error> {
    let sql_family = renderer.sql_family();
    let schema_name = database_info.connection_info().schema_name().to_string();
    let differ = SqlSchemaDiffer {
        previous: current_schema,
        next: next_schema,
        database_info,
        flavour: renderer,
    };

    match step {
        SqlMigrationStep::RedefineTables { names } => Ok(renderer.render_redefine_tables(names, differ, database_info)),
        SqlMigrationStep::CreateEnum(create_enum) => Ok(renderer.render_create_enum(create_enum)),
        SqlMigrationStep::DropEnum(drop_enum) => Ok(renderer.render_drop_enum(drop_enum)),
        SqlMigrationStep::AlterEnum(alter_enum) => renderer.render_alter_enum(alter_enum, &differ, &schema_name),
        SqlMigrationStep::CreateTable(CreateTable { table }) => {
            let table = next_schema
                .table_walker(&table.name)
                .expect("CreateTable referring to an unknown table.");

            Ok(vec![renderer.render_create_table(&table, &schema_name)?])
        }
        SqlMigrationStep::DropTable(DropTable { name }) => Ok(renderer.render_drop_table(name, &schema_name)),
        SqlMigrationStep::RenameTable { name, new_name } => {
            let new_name = match sql_family {
                SqlFamily::Sqlite => renderer.quote(new_name).to_string(),
                _ => renderer.quote_with_schema(&schema_name, &new_name).to_string(),
            };
            Ok(vec![format!(
                "ALTER TABLE {} RENAME TO {}",
                renderer.quote_with_schema(&schema_name, &name),
                new_name
            )])
        }
        SqlMigrationStep::AddForeignKey(add_foreign_key) => {
            Ok(vec![renderer.render_add_foreign_key(add_foreign_key, &schema_name)])
        }
        SqlMigrationStep::DropForeignKey(DropForeignKey { table, constraint_name }) => match sql_family {
            SqlFamily::Mysql => Ok(vec![format!(
                "ALTER TABLE {table} DROP FOREIGN KEY {constraint_name}",
                table = renderer.quote_with_schema(&schema_name, table),
                constraint_name = Quoted::mysql_ident(constraint_name),
            )]),
            SqlFamily::Postgres => Ok(vec![format!(
                "ALTER TABLE {table} DROP CONSTRAINT {constraint_name}",
                table = renderer.quote_with_schema(&schema_name, table),
                constraint_name = Quoted::postgres_ident(constraint_name),
            )]),
            SqlFamily::Sqlite => Ok(Vec::new()),
            SqlFamily::Mssql => todo!("Greetings from Redmond"),
        },

        SqlMigrationStep::AlterTable(alter_table) => {
            Ok(renderer.render_alter_table(alter_table, database_info, &differ))
        }
        SqlMigrationStep::CreateIndex(create_index) => {
            Ok(vec![renderer.render_create_index(create_index, database_info)])
        }
        SqlMigrationStep::DropIndex(drop_index) => Ok(vec![renderer.render_drop_index(drop_index, database_info)]),
        SqlMigrationStep::AlterIndex(alter_index) => {
            renderer.render_alter_index(alter_index, database_info, current_schema)
        }
    }
}

pub(crate) fn render_create_index(
    renderer: &dyn SqlFlavour,
    schema_name: &str,
    table_name: &str,
    index: &Index,
    sql_family: SqlFamily,
) -> String {
    let Index { name, columns, tpe } = index;
    let index_type = match tpe {
        IndexType::Unique => "UNIQUE ",
        IndexType::Normal => "",
    };
    let index_name = match sql_family {
        SqlFamily::Sqlite => renderer.quote_with_schema(schema_name, &name).to_string(),
        _ => renderer.quote(&name).to_string(),
    };
    let table_reference = match sql_family {
        SqlFamily::Sqlite => renderer.quote(table_name).to_string(),
        _ => renderer.quote_with_schema(schema_name, table_name).to_string(),
    };
    let columns = columns.iter().map(|c| renderer.quote(c));

    format!(
        "CREATE {index_type}INDEX {index_name} ON {table_reference}({columns})",
        index_type = index_type,
        index_name = index_name,
        table_reference = table_reference,
        columns = columns.join(", ")
    )
}

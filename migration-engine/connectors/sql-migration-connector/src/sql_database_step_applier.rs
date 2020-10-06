use crate::{
    database_info::DatabaseInfo,
    sql_migration::{CreateTable, DropTable, SqlMigration, SqlMigrationStep},
    sql_schema_differ::SqlSchemaDiffer,
    Component, SqlFlavour,
};
use migration_connector::{
    ConnectorError, ConnectorResult, DatabaseMigrationMarker, DatabaseMigrationStepApplier,
    DestructiveChangeDiagnostics, PrettyDatabaseMigrationStep,
};
use sql_schema_describer::{walkers::SqlSchemaExt, SqlSchema};

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
    #[tracing::instrument(skip(self, database_migration))]
    async fn apply_step(&self, database_migration: &SqlMigration, index: usize) -> ConnectorResult<bool> {
        self.apply_next_step(
            &database_migration.steps,
            index,
            self.flavour(),
            &database_migration.before,
            &database_migration.after,
        )
        .await
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

    fn render_script(&self, database_migration: &SqlMigration, diagnostics: &DestructiveChangeDiagnostics) -> String {
        if database_migration.is_empty() {
            return "-- This is an empty migration.".to_string();
        }

        let mut script = String::with_capacity(40 * database_migration.steps.len());

        // Note: it would be much nicer if we could place the warnings next to
        // the SQL for the steps that triggered them.
        if diagnostics.has_warnings() || diagnostics.unexecutable_migrations.len() > 0 {
            script.push_str("/*\n  Warnings:\n\n");

            for warning in &diagnostics.warnings {
                script.push_str("  - ");
                script.push_str(&warning.description);
                script.push_str("\n");
            }

            for unexecutable in &diagnostics.unexecutable_migrations {
                script.push_str("  - ");
                script.push_str(&unexecutable.description);
                script.push_str("\n");
            }

            script.push_str("\n*/\n")
        }

        for step in &database_migration.steps {
            let statements: Vec<String> = render_raw_sql(
                step,
                self.flavour(),
                self.database_info(),
                &database_migration.before,
                &database_migration.after,
            )
            .unwrap();

            script.push_str("-- ");
            script.push_str(step.description());
            script.push_str("\n");

            for statement in statements {
                script.push_str(&statement);
                script.push_str(";\n");
            }
        }

        script
    }

    async fn apply_script(&self, script: &str) -> ConnectorResult<()> {
        self.conn().raw_cmd(script).await
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
    ) -> ConnectorResult<bool> {
        let has_this_one = steps.get(index).is_some();

        if !has_this_one {
            return Ok(false);
        }

        let step = &steps[index];
        tracing::debug!(?step);

        for sql_string in render_raw_sql(&step, renderer, self.database_info(), current_schema, next_schema)
            .map_err(|err| ConnectorError::generic(err))?
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
    let differ = SqlSchemaDiffer {
        previous: current_schema,
        next: next_schema,
        database_info,
        flavour: renderer,
    };

    match step {
        SqlMigrationStep::RedefineTables { names } => Ok(renderer.render_redefine_tables(names, differ)),
        SqlMigrationStep::CreateEnum(create_enum) => Ok(renderer.render_create_enum(create_enum)),
        SqlMigrationStep::DropEnum(drop_enum) => Ok(renderer.render_drop_enum(drop_enum)),
        SqlMigrationStep::AlterEnum(alter_enum) => renderer.render_alter_enum(alter_enum, &differ),
        SqlMigrationStep::CreateTable(CreateTable { table }) => {
            let table = next_schema
                .table_walker(&table.name)
                .expect("CreateTable referring to an unknown table.");

            Ok(vec![renderer.render_create_table(&table)])
        }
        SqlMigrationStep::DropTable(DropTable { name }) => Ok(renderer.render_drop_table(name)),
        SqlMigrationStep::RenameTable { name, new_name } => Ok(vec![renderer.render_rename_table(name, new_name)]),
        SqlMigrationStep::AddForeignKey(add_foreign_key) => Ok(vec![renderer.render_add_foreign_key(add_foreign_key)]),
        SqlMigrationStep::DropForeignKey(drop_foreign_key) => {
            Ok(vec![renderer.render_drop_foreign_key(drop_foreign_key)])
        }
        SqlMigrationStep::AlterTable(alter_table) => Ok(renderer.render_alter_table(alter_table, &differ)),
        SqlMigrationStep::CreateIndex(create_index) => Ok(vec![renderer.render_create_index(create_index)]),
        SqlMigrationStep::DropIndex(drop_index) => Ok(vec![renderer.render_drop_index(drop_index)]),
        SqlMigrationStep::AlterIndex(alter_index) => {
            renderer.render_alter_index(alter_index, database_info, current_schema)
        }
    }
}

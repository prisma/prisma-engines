use crate::{
    migration_pair::MigrationPair,
    sql_migration::{SqlMigration, SqlMigrationStep},
    SqlFlavour, SqlSchemaConnector,
};
use schema_connector::{ConnectorResult, DestructiveChangeDiagnostics, Migration};
use sql_schema_describer::SqlSchema;
use tracing_futures::Instrument;

#[tracing::instrument(skip(flavour, migration))]
pub(crate) async fn apply_migration(
    migration: &Migration,
    flavour: &mut (dyn SqlFlavour + Send + Sync),
) -> ConnectorResult<u32> {
    let migration: &SqlMigration = migration.downcast_ref();
    tracing::debug!("{} steps to execute", migration.steps.len());

    for step in &migration.steps {
        for sql_string in render_raw_sql(step, flavour, MigrationPair::new(&migration.before, &migration.after)) {
            assert!(!sql_string.is_empty());
            let span = tracing::info_span!("migration_step", ?step);
            flavour.raw_cmd(&sql_string).instrument(span).await?;
        }
    }

    Ok(migration.steps.len() as u32)
}

#[tracing::instrument(skip(migration, flavour))]
pub(crate) fn render_script(
    migration: &Migration,
    diagnostics: &DestructiveChangeDiagnostics,
    flavour: &(dyn SqlFlavour + Send + Sync),
) -> ConnectorResult<String> {
    let migration: &SqlMigration = migration.downcast_ref();
    if migration.steps.is_empty() {
        return Ok("-- This is an empty migration.".to_owned());
    }

    let mut script = String::with_capacity(40 * migration.steps.len());

    // Note: it would be much nicer if we could place the warnings next to
    // the SQL for the steps that triggered them.
    if diagnostics.has_warnings() || !diagnostics.unexecutable_migrations.is_empty() {
        script.push_str("/*\n  Warnings:\n\n");

        for warning in &diagnostics.warnings {
            script.push_str("  - ");
            script.push_str(&warning.description);
            script.push('\n');
        }

        for unexecutable in &diagnostics.unexecutable_migrations {
            script.push_str("  - ");
            script.push_str(&unexecutable.description);
            script.push('\n');
        }

        script.push_str("\n*/\n")
    }

    // Whether we are on the first *rendered* step, to avoid printing a
    // newline before it. This can't be `enumerate()` on the loop because
    // some steps don't render anything.
    let mut is_first_step = true;

    if let Some(begin) = flavour.render_begin_transaction() {
        script.push_str(begin);
        script.push('\n');
    }

    for step in &migration.steps {
        let statements: Vec<String> =
            render_raw_sql(step, flavour, MigrationPair::new(&migration.before, &migration.after));

        if !statements.is_empty() {
            if is_first_step {
                is_first_step = false;
            } else {
                script.push('\n');
            }

            // We print a newline *before* migration steps and not after,
            // because we do not want two newlines at the end of the file:
            // many editors will remove trailing newlines, and automatically
            // edit the migration.
            script.push_str("-- ");
            script.push_str(step.description());
            script.push('\n');

            for statement in statements {
                script.push_str(&statement);
                script.push_str(";\n");
            }
        }
    }

    if let Some(commit) = flavour.render_commit_transaction() {
        script.push('\n');
        script.push_str(commit);
    }

    Ok(script)
}

#[tracing::instrument(skip(script, connector))]
pub(crate) async fn apply_script(
    migration_name: &str,
    script: &str,
    connector: &mut SqlSchemaConnector,
) -> ConnectorResult<()> {
    connector
        .host
        .print(&format!("Applying migration `{migration_name}`\n"))
        .await?;
    connector.flavour.scan_migration_script(script);
    connector.flavour.apply_migration_script(migration_name, script).await
}

fn render_raw_sql(
    step: &SqlMigrationStep,
    renderer: &(dyn SqlFlavour + Send + Sync),
    schemas: MigrationPair<&SqlSchema>,
) -> Vec<String> {
    match step {
        SqlMigrationStep::AlterSequence(sequence_ids, changes) => {
            renderer.render_alter_sequence(*sequence_ids, *changes, schemas)
        }
        SqlMigrationStep::AlterPrimaryKey(table_id) => renderer.render_alter_primary_key(schemas.walk(*table_id)),
        SqlMigrationStep::AlterEnum(alter_enum) => renderer.render_alter_enum(alter_enum, schemas),
        SqlMigrationStep::RedefineTables(redefine_tables) => renderer.render_redefine_tables(redefine_tables, schemas),
        SqlMigrationStep::CreateEnum(enum_id) => renderer.render_create_enum(schemas.next.walk(*enum_id)),
        SqlMigrationStep::CreateSchema(namespace_id) => {
            vec![renderer.render_create_namespace(schemas.next.walk(*namespace_id))]
        }
        SqlMigrationStep::DropEnum(enum_id) => renderer.render_drop_enum(schemas.previous.walk(*enum_id)),
        SqlMigrationStep::CreateTable { table_id } => {
            let table = schemas.next.walk(*table_id);

            vec![renderer.render_create_table(table)]
        }
        SqlMigrationStep::DropTable { table_id } => {
            let table = schemas.previous.walk(*table_id);

            renderer.render_drop_table(table.namespace(), table.name())
        }
        SqlMigrationStep::RedefineIndex { index } => renderer.render_drop_and_recreate_index(schemas.walk(*index)),
        SqlMigrationStep::AddForeignKey { foreign_key_id } => {
            let foreign_key = schemas.next.walk(*foreign_key_id);
            vec![renderer.render_add_foreign_key(foreign_key)]
        }
        SqlMigrationStep::DropForeignKey { foreign_key_id } => {
            let foreign_key = schemas.previous.walk(*foreign_key_id);
            vec![renderer.render_drop_foreign_key(foreign_key)]
        }
        SqlMigrationStep::AlterTable(alter_table) => renderer.render_alter_table(alter_table, schemas),
        SqlMigrationStep::CreateIndex {
            table_id: _,
            index_id,
            from_drop_and_recreate: _,
        } => vec![renderer.render_create_index(schemas.next.walk(*index_id))],
        SqlMigrationStep::DropIndex { index_id } => {
            vec![renderer.render_drop_index(schemas.previous.walk(*index_id))]
        }
        SqlMigrationStep::RenameIndex { index } => renderer.render_rename_index(schemas.walk(*index)),
        SqlMigrationStep::DropView(drop_view) => {
            let view = schemas.previous.walk(drop_view.view_id);

            vec![renderer.render_drop_view(view)]
        }
        SqlMigrationStep::DropUserDefinedType(drop_udt) => {
            let udt = schemas.previous.walk(drop_udt.udt_id);

            vec![renderer.render_drop_user_defined_type(&udt)]
        }
        SqlMigrationStep::RenameForeignKey { foreign_key_id } => {
            let fks = schemas.walk(*foreign_key_id);
            vec![renderer.render_rename_foreign_key(fks)]
        }
        #[cfg(feature = "postgresql")]
        SqlMigrationStep::CreateExtension(create_extension) => {
            renderer.render_create_extension(create_extension, schemas.next)
        }
        #[cfg(feature = "postgresql")]
        SqlMigrationStep::AlterExtension(alter_extension) => {
            renderer.render_alter_extension(alter_extension, MigrationPair::new(schemas.previous, schemas.next))
        }
        #[cfg(feature = "postgresql")]
        SqlMigrationStep::DropExtension(drop_extension) => {
            renderer.render_drop_extension(drop_extension, schemas.previous)
        }
    }
}

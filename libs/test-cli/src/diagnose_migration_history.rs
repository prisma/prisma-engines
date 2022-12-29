use migration_core::commands::DiagnoseMigrationHistoryInput;

use crate::DiagnoseMigrationHistory;

impl DiagnoseMigrationHistory {
    pub(crate) async fn execute(&self) -> anyhow::Result<()> {
        // TODO: accept namespaces as input argument
        let input = DiagnoseMigrationHistoryInput {
            migrations_directory_path: self.migrations_directory_path.clone(),
            opt_in_to_shadow_database: true,
        };
        let schema = crate::read_datamodel_from_file(&self.schema_path)?;

        let namespaces: Vec<String> = vec![];
        let engine = migration_core::migration_api(Some(schema), namespaces, None)?;

        let output = engine.diagnose_migration_history(input).await?;

        eprintln!("{:#?}", output);

        Ok(())
    }
}

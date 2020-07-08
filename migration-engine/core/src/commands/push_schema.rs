use super::{CommandError, MigrationCommand};
use crate::parse_datamodel;
use datamodel::Datamodel;
use migration_connector::{DatabaseMigrationMarker, MigrationConnector};
use serde::{Deserialize, Serialize};

pub struct PushSchemaCommand<'a> {
    input: &'a PushSchemaInput,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for PushSchemaCommand<'a> {
    type Input = PushSchemaInput;
    type Output = PushSchemaOutput;

    async fn execute<C, D>(
        input: &Self::Input,
        engine: &crate::migration_engine::MigrationEngine<C, D>,
    ) -> super::CommandResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + Send + Sync + 'static,
    {
        let connector = engine.connector();
        let schema = parse_datamodel(&input.schema)?;

        connector.push_schema(&schema).await?;

        Ok(PushSchemaOutput {})
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSchemaInput {
    /// The prisma schema.
    pub schema: String,
    /// Push the schema ignoring destructive change warnings.
    pub force: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSchemaOutput {}

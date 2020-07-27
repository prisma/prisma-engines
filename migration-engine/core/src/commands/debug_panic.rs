use super::{CommandResult, MigrationCommand};
use crate::migration_engine::MigrationEngine;

pub struct DebugPanicCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for DebugPanicCommand {
    type Input = ();
    type Output = ();

    async fn execute<C, D>(_input: &Self::Input, _engine: &MigrationEngine<C, D>) -> CommandResult<Self::Output>
    where
        C: migration_connector::MigrationConnector<DatabaseMigration = D>,
        D: migration_connector::DatabaseMigrationMarker + Send + Sync + 'static,
    {
        panic!("This is the debugPanic artificial panic")
    }
}

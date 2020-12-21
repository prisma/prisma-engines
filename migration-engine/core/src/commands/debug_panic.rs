use super::MigrationCommand;
use crate::{core_error::CoreResult, migration_engine::MigrationEngine};
use migration_connector::MigrationConnector;

/// Make the migration engine crash. This is useful only for debugging error handling in clients.
pub struct DebugPanicCommand;

#[async_trait::async_trait]
impl<'a> MigrationCommand for DebugPanicCommand {
    type Input = ();
    type Output = ();

    async fn execute<C: MigrationConnector>(
        _input: &Self::Input,
        _engine: &MigrationEngine<C>,
    ) -> CoreResult<Self::Output> {
        panic!("This is the debugPanic artificial panic")
    }
}

use crate::{migration_engine::MigrationEngine, CoreResult};
use migration_connector::*;
use serde::{de::DeserializeOwned, Serialize};

/// The implementation of an RPC command exposed by the migration engine.
#[async_trait::async_trait]
pub trait MigrationCommand {
    /// The input parameters to the command.
    type Input: DeserializeOwned;
    /// The response shape of the command.
    type Output: Serialize + 'static;

    /// Handle the input, producing the response or an error.
    async fn execute<C: MigrationConnector>(
        input: &Self::Input,
        engine: &MigrationEngine<C>,
    ) -> CoreResult<Self::Output>;
}

use std::collections::HashMap;

use super::MigrationCommand;
use crate::{api::MigrationApi, core_error::CoreResult};
use migration_connector::MigrationConnector;
use serde::Serialize;

/// Method called at the beginning of `migrate dev` to decide the course of
/// action based on the current state of the workspace.
pub struct DevDiagnosticCommand;

/// Alias for the empty `devDiagnostic` input.
pub type DevDiagnosticInput = HashMap<(), ()>;

/// The response type for `devDiagnostic`.
#[derive(Debug, Serialize)]
pub struct DevDiagnosticOutput {
    test: String,
}

#[async_trait::async_trait]
impl<'a> MigrationCommand for DevDiagnosticCommand {
    type Input = DevDiagnosticInput;
    type Output = DevDiagnosticOutput;

    async fn execute<C: MigrationConnector>(
        _input: &Self::Input,
        _engine: &MigrationApi<C>,
    ) -> CoreResult<Self::Output> {
        panic!("This is the debugPanic artificial panic")
    }
}

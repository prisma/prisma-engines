use crate::migration_engine::MigrationEngine;
use crate::{commands::command::*, CoreResult};
use datamodel::ast::SchemaAst;
use migration_connector::*;
use serde::{Deserialize, Serialize};
use tracing::debug;

pub struct CalculateDatamodelCommand;

#[async_trait::async_trait]
impl MigrationCommand for CalculateDatamodelCommand {
    type Input = CalculateDatamodelInput;
    type Output = CalculateDatamodelOutput;

    async fn execute<C, D>(input: &Self::Input, engine: &MigrationEngine<C, D>) -> CoreResult<Self::Output>
    where
        C: MigrationConnector<DatabaseMigration = D>,
        D: DatabaseMigrationMarker + 'static,
    {
        debug!("{:?}", input);

        let base_datamodel = SchemaAst::empty();
        let datamodel = engine.datamodel_calculator().infer(&base_datamodel, &input.steps)?;

        Ok(CalculateDatamodelOutput {
            datamodel: datamodel::render_schema_ast_to_string(&datamodel).unwrap(),
        })
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatamodelInput {
    pub steps: Vec<MigrationStep>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculateDatamodelOutput {
    pub datamodel: String,
}

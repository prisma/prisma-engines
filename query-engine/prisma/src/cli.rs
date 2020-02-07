use std::{convert::TryFrom, fs::File, io::Read, sync::Arc};

use serde::Deserialize;

use datamodel::json::dmmf::Datamodel;
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QuerySchemaBuilder,
};
use std::collections::HashMap;

use crate::context::PrismaContext;
use crate::error::PrismaError;
use crate::request_handlers::{graphql::*, PrismaRequest, RequestHandler};
use crate::{
    data_model_loader::{load_configuration, load_data_model_components},
    dmmf, PrismaResult,
};
use crate::{CliOpt, PrismaOpt, Subcommand};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DmmfToDmlInput {
    pub dmmf: Datamodel,
    pub config: serde_json::Value,
}

pub struct ExecuteRequest {
    query: String,
    force_transactions: bool,
    enable_raw_queries: bool,
}

pub struct DmmfRequest {
    build_mode: BuildMode,
    enable_raw_queries: bool,
}

pub enum CliCommand {
    Dmmf(DmmfRequest),
    DmmfToDml(DmmfToDmlInput),
    GetConfig(String),
    ExecuteRequest(ExecuteRequest),
}

impl TryFrom<&PrismaOpt> for CliCommand {
    type Error = PrismaError;

    fn try_from(opts: &PrismaOpt) -> crate::PrismaResult<CliCommand> {
        match opts.subcommand {
            None => Err(PrismaError::InvocationError(String::from("cli subcommand not present"))),
            Some(Subcommand::Cli(ref cliopts)) => match cliopts {
                CliOpt::Dmmf => {
                    let build_mode = if opts.legacy {
                        BuildMode::Legacy
                    } else {
                        BuildMode::Modern
                    };

                    Ok(CliCommand::Dmmf(DmmfRequest {
                        build_mode,
                        enable_raw_queries: opts.enable_raw_queries,
                    }))
                }
                CliOpt::DmmfToDml(input) => {
                    let file = File::open(&input.path).expect("File should open read only");
                    let input = serde_json::from_reader(file).expect("File should be proper JSON");

                    Ok(CliCommand::DmmfToDml(input))
                }
                CliOpt::GetConfig(input) => {
                    let mut file = File::open(&input.path).expect("File should open read only");
                    let mut datamodel = String::new();

                    file.read_to_string(&mut datamodel).expect("Couldn't read file");
                    Ok(CliCommand::GetConfig(datamodel))
                }
                CliOpt::ExecuteRequest(input) => Ok(CliCommand::ExecuteRequest(ExecuteRequest {
                    query: input.query.clone(),
                    force_transactions: opts.always_force_transactions,
                    enable_raw_queries: opts.enable_raw_queries,
                })),
            },
        }
    }
}

impl CliCommand {
    pub async fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(request) => Self::dmmf(request),
            CliCommand::DmmfToDml(input) => Self::dmmf_to_dml(input),
            CliCommand::GetConfig(input) => Self::get_config(input),
            CliCommand::ExecuteRequest(request) => Self::execute_request(request).await,
        }
    }

    fn dmmf(request: DmmfRequest) -> PrismaResult<()> {
        let (v2components, template) = load_data_model_components()?;

        // temporary code duplication
        let internal_data_model = template.build("".into());
        let capabilities = SupportedCapabilities::empty();

        let schema_builder = QuerySchemaBuilder::new(
            &internal_data_model,
            &capabilities,
            request.build_mode,
            request.enable_raw_queries,
        );

        let query_schema: QuerySchemaRef = Arc::new(schema_builder.build());

        let dmmf = dmmf::render_dmmf(&v2components.datamodel, query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{}", serialized);

        Ok(())
    }

    fn dmmf_to_dml(input: DmmfToDmlInput) -> PrismaResult<()> {
        let datamodel = datamodel::json::dmmf::schema_from_dmmf(&input.dmmf);
        let config = datamodel::json::mcf::config_from_mcf_json_value(input.config);
        let serialized = datamodel::render_datamodel_and_config_to_string(&datamodel, &config)?;

        println!("{}", serialized);

        Ok(())
    }

    fn get_config(input: String) -> PrismaResult<()> {
        let config = load_configuration(&input)?;
        let json = datamodel::json::mcf::config_to_mcf_json_value(&config);
        let serialized = serde_json::to_string(&json)?;

        println!("{}", serialized);

        Ok(())
    }

    async fn execute_request(request: ExecuteRequest) -> PrismaResult<()> {
        let decoded = base64::decode(&request.query)?;
        let decoded_request = String::from_utf8(decoded)?;

        let ctx = PrismaContext::builder()
            .legacy(true)
            .force_transactions(request.force_transactions)
            .enable_raw_queries(request.enable_raw_queries)
            .build()
            .await?;

        let req = PrismaRequest {
            body: serde_json::from_str(&decoded_request).unwrap(),
            headers: HashMap::new(),
            path: String::new(),
        };

        let response = GraphQlRequestHandler.handle(req, &Arc::new(ctx)).await;
        let response = serde_json::to_string(&response).unwrap();

        let encoded_response = base64::encode(&response);
        println!("Response: {}", encoded_response); // reason for prefix is explained in TestServer.scala

        Ok(())
    }
}

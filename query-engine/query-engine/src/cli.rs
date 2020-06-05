use crate::{
    context::PrismaContext,
    dmmf,
    error::PrismaError,
    opt::{CliOpt, PrismaOpt, Subcommand},
    request_handlers::{graphql::*, PrismaRequest, RequestHandler},
    PrismaResult,
};
use datamodel::{Configuration, Datamodel};
use prisma_models::DatamodelConverter;
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QuerySchemaBuilder,
};
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

pub struct ExecuteRequest {
    legacy: bool,
    query: String,
    datamodel: Datamodel,
    config: Configuration,
    enable_raw_queries: bool,
}

pub struct DmmfRequest {
    datamodel: Datamodel,
    build_mode: BuildMode,
    enable_raw_queries: bool,
}

pub struct GetConfigRequest {
    config: Configuration,
}

pub enum CliCommand {
    Dmmf(DmmfRequest),
    GetConfig(GetConfigRequest),
    ExecuteRequest(ExecuteRequest),
}

impl TryFrom<&PrismaOpt> for CliCommand {
    type Error = PrismaError;

    fn try_from(opts: &PrismaOpt) -> crate::PrismaResult<CliCommand> {
        let subcommand = opts
            .subcommand
            .as_ref()
            .ok_or_else(|| PrismaError::InvocationError(String::from("cli subcommand not present")))?;

        match subcommand {
            Subcommand::Cli(ref cliopts) => match cliopts {
                CliOpt::Dmmf => {
                    let build_mode = if opts.legacy {
                        BuildMode::Legacy
                    } else {
                        BuildMode::Modern
                    };

                    Ok(CliCommand::Dmmf(DmmfRequest {
                        datamodel: opts.datamodel(true)?,
                        build_mode,
                        enable_raw_queries: opts.enable_raw_queries,
                    }))
                }
                CliOpt::GetConfig(input) => Ok(CliCommand::GetConfig(GetConfigRequest {
                    config: opts.configuration(input.ignore_env_var_errors)?,
                })),
                CliOpt::ExecuteRequest(input) => Ok(CliCommand::ExecuteRequest(ExecuteRequest {
                    query: input.query.clone(),
                    enable_raw_queries: opts.enable_raw_queries,
                    legacy: input.legacy,
                    datamodel: opts.datamodel(false)?,
                    config: opts.configuration(false)?,
                })),
            },
        }
    }
}

impl CliCommand {
    pub async fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(request) => Self::dmmf(request),
            CliCommand::GetConfig(input) => Self::get_config(input.config),
            CliCommand::ExecuteRequest(request) => Self::execute_request(request).await,
        }
    }

    fn dmmf(request: DmmfRequest) -> PrismaResult<()> {
        let template = DatamodelConverter::convert(&request.datamodel);

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

        let dmmf = dmmf::render_dmmf(&request.datamodel, query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{}", serialized);

        Ok(())
    }

    fn get_config(config: Configuration) -> PrismaResult<()> {
        let json = datamodel::json::mcf::config_to_mcf_json_value(&config);
        let serialized = serde_json::to_string(&json)?;

        println!("{}", serialized);

        Ok(())
    }

    async fn execute_request(request: ExecuteRequest) -> PrismaResult<()> {
        let decoded = base64::decode(&request.query)?;
        let decoded_request = String::from_utf8(decoded)?;

        let ctx = PrismaContext::builder(request.config, request.datamodel)
            .legacy(request.legacy)
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

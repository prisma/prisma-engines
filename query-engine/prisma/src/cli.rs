use crate::{
    configuration,
    context::PrismaContext,
    dmmf,
    error::PrismaError,
    request_handlers::{graphql::*, PrismaRequest, RequestHandler},
    PrismaResult, {CliOpt, PrismaOpt, Subcommand},
};
use prisma_models::DatamodelConverter;
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QuerySchemaBuilder,
};
use std::{collections::HashMap, convert::TryFrom, sync::Arc};

#[derive(Debug)]
pub struct ExecuteRequest {
    legacy: bool,
    query: String,
    datamodel: String,
    force_transactions: bool,
    enable_raw_queries: bool,
    overwrite_datasources: Option<String>,
}

#[derive(Debug)]
pub struct DmmfRequest {
    datamodel: String,
    build_mode: BuildMode,
    enable_raw_queries: bool,
    overwrite_datasources: Option<String>,
}

#[derive(Debug)]
pub struct GetConfigRequest {
    datamodel: String,
    overwrite_datasources: Option<String>,
}

pub enum CliCommand {
    Dmmf(DmmfRequest),
    GetConfig(GetConfigRequest),
    ExecuteRequest(ExecuteRequest),
}

impl TryFrom<&PrismaOpt> for CliCommand {
    type Error = PrismaError;

    fn try_from(opts: &PrismaOpt) -> crate::PrismaResult<CliCommand> {
        let subcommand = opts.subcommand.clone().ok_or_else(|| {
            PrismaError::InvocationError(String::from("cli subcommand not present"))
        })?;

        let datamodel = opts
            .datamodel
            .clone()
            .xor(opts.datamodel_path.clone())
            .expect("Datamodel should be provided either as path or base64-encoded string.");

        match subcommand {
            Subcommand::Cli(ref cliopts) => match cliopts {
                CliOpt::Dmmf => {
                    let build_mode = if opts.legacy {
                        BuildMode::Legacy
                    } else {
                        BuildMode::Modern
                    };

                    Ok(CliCommand::Dmmf(DmmfRequest {
                        datamodel,
                        build_mode,
                        enable_raw_queries: opts.enable_raw_queries,
                        overwrite_datasources: opts.overwrite_datasources.clone(),
                    }))
                }
                CliOpt::GetConfig => Ok(CliCommand::GetConfig(GetConfigRequest {
                    datamodel,
                    overwrite_datasources: opts.overwrite_datasources.clone(),
                })),
                CliOpt::ExecuteRequest(input) => Ok(CliCommand::ExecuteRequest(ExecuteRequest {
                    query: input.query.clone(),
                    force_transactions: opts.always_force_transactions,
                    overwrite_datasources: opts.overwrite_datasources.clone(),
                    enable_raw_queries: opts.enable_raw_queries,
                    legacy: input.legacy,
                    datamodel,
                })),
            },
        }
    }
}

impl CliCommand {
    pub async fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(request) => Self::dmmf(request),
            CliCommand::GetConfig(input) => {
                Self::get_config(input.datamodel, input.overwrite_datasources)
            }
            CliCommand::ExecuteRequest(request) => Self::execute_request(request).await,
        }
    }

    fn dmmf(request: DmmfRequest) -> PrismaResult<()> {
        let dm = datamodel::parse_datamodel_and_ignore_env_errors(&request.datamodel)
            .map_err(|errors| PrismaError::ConversionError(errors, request.datamodel.clone()))?;

        let template = DatamodelConverter::convert(&dm);

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

        let dmmf = dmmf::render_dmmf(&dm, query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{}", serialized);

        Ok(())
    }

    fn get_config(datamodel: String, overwrite_datasources: Option<String>) -> PrismaResult<()> {
        let config = configuration::load(&datamodel, overwrite_datasources, true)?;
        let json = datamodel::json::mcf::config_to_mcf_json_value(&config);
        let serialized = serde_json::to_string(&json)?;

        println!("{}", serialized);

        Ok(())
    }

    async fn execute_request(request: ExecuteRequest) -> PrismaResult<()> {
        let decoded = base64::decode(&request.query)?;
        let decoded_request = String::from_utf8(decoded)?;

        let ctx = PrismaContext::builder(request.datamodel)
            .legacy(request.legacy)
            .force_transactions(request.force_transactions)
            .enable_raw_queries(request.enable_raw_queries)
            .overwrite_datasources(request.overwrite_datasources)
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

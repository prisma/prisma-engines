use crate::{
    context::PrismaContext,
    opt::{CliOpt, PrismaOpt, Subcommand},
    PrismaResult,
};

use datamodel::diagnostics::ValidatedConfiguration;
use datamodel::{Configuration, Datamodel};
use datamodel_connector::ConnectorCapabilities;
use prisma_models::DatamodelConverter;
use query_core::{schema::QuerySchemaRef, schema_builder, BuildMode};
use request_handlers::{dmmf, GraphQlHandler};
use std::sync::Arc;

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
    config: Configuration,
}

pub struct GetConfigRequest {
    config: ValidatedConfiguration,
    ignore_env_var_errors: bool,
}

pub enum CliCommand {
    Dmmf(DmmfRequest),
    GetConfig(GetConfigRequest),
    ExecuteRequest(ExecuteRequest),
}

impl CliCommand {
    /// Create a CLI command from a `PrismaOpt` instance.
    pub(crate) fn from_opt(opts: &PrismaOpt) -> crate::PrismaResult<Option<CliCommand>> {
        let subcommand = opts.subcommand.as_ref();
        let subcommand = match subcommand {
            Some(cmd) => cmd,
            None => return Ok(None),
        };

        match subcommand {
            Subcommand::Cli(ref cliopts) => match cliopts {
                CliOpt::Dmmf => {
                    let build_mode = if opts.legacy {
                        BuildMode::Legacy
                    } else {
                        BuildMode::Modern
                    };

                    Ok(Some(CliCommand::Dmmf(DmmfRequest {
                        datamodel: opts.datamodel()?,
                        build_mode,
                        enable_raw_queries: opts.enable_raw_queries,
                        config: opts.configuration(true)?.subject,
                    })))
                }
                CliOpt::GetConfig(input) => Ok(Some(CliCommand::GetConfig(GetConfigRequest {
                    config: opts.configuration(input.ignore_env_var_errors)?,
                    ignore_env_var_errors: input.ignore_env_var_errors,
                }))),
                CliOpt::ExecuteRequest(input) => Ok(Some(CliCommand::ExecuteRequest(ExecuteRequest {
                    query: input.query.clone(),
                    enable_raw_queries: opts.enable_raw_queries,
                    legacy: input.legacy,
                    datamodel: opts.datamodel()?,
                    config: opts.configuration(false)?.subject,
                }))),
            },
        }
    }

    pub async fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(request) => Self::dmmf(request).await,
            CliCommand::GetConfig(input) => Self::get_config(input),
            CliCommand::ExecuteRequest(request) => Self::execute_request(request).await,
        }
    }

    async fn dmmf(request: DmmfRequest) -> PrismaResult<()> {
        let template = DatamodelConverter::convert(&request.datamodel);

        let capabilities = match request.config.datasources.first() {
            Some(datasource) => datasource.capabilities(),
            None => ConnectorCapabilities::empty(),
        };

        // temporary code duplication
        let internal_data_model = template.build("".into());
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            request.build_mode,
            request.enable_raw_queries,
            capabilities,
            request.config.preview_features().cloned().collect(),
        ));

        let dmmf = dmmf::render_dmmf(&request.datamodel, query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{}", serialized);

        Ok(())
    }

    fn get_config(mut req: GetConfigRequest) -> PrismaResult<()> {
        let config = &mut req.config;

        if !req.ignore_env_var_errors {
            config.subject.resolve_datasource_urls_from_env(&[])?;
        }

        let json = datamodel::json::mcf::config_to_mcf_json_value(&config);
        let serialized = serde_json::to_string(&json)?;

        println!("{}", serialized);

        Ok(())
    }

    async fn execute_request(request: ExecuteRequest) -> PrismaResult<()> {
        let decoded = base64::decode(&request.query)?;
        let decoded_request = String::from_utf8(decoded)?;

        request.config.validate_that_one_datasource_is_provided()?;

        let cx = PrismaContext::builder(request.config, request.datamodel)
            .legacy(request.legacy)
            .enable_raw_queries(request.enable_raw_queries)
            .build()
            .await?;

        let cx = Arc::new(cx);

        let handler = GraphQlHandler::new(&*cx.executor, cx.query_schema());
        let res = handler.handle(serde_json::from_str(&decoded_request)?).await;
        let res = serde_json::to_string(&res).unwrap();

        let encoded_response = base64::encode(&res);
        println!("Response: {}", encoded_response); // reason for prefix is explained in TestServer.scala

        Ok(())
    }
}

use crate::{
    context::PrismaContext,
    opt::{CliOpt, PrismaOpt, Subcommand},
    PrismaResult,
};
use prisma_models::InternalDataModelBuilder;
use psl::{dml::Datamodel, Configuration};
use query_core::{schema::QuerySchemaRef, schema_builder};
use request_handlers::{dmmf, GraphQlHandler};
use std::{env, sync::Arc};

pub struct ExecuteRequest {
    query: String,
    datamodel: Datamodel,
    config: Configuration,
    enable_raw_queries: bool,
}

pub struct DmmfRequest {
    datamodel: Datamodel,
    enable_raw_queries: bool,
    config: Configuration,
}

pub struct GetConfigRequest {
    config: Configuration,
    ignore_env_var_errors: bool,
}

pub struct DebugPanicRequest {
    message: Option<String>,
}

pub enum CliCommand {
    Dmmf(DmmfRequest),
    GetConfig(GetConfigRequest),
    ExecuteRequest(ExecuteRequest),
    DebugPanic(DebugPanicRequest),
}

impl CliCommand {
    /// Create a CLI command from a `PrismaOpt` instance.
    pub fn from_opt(opts: &PrismaOpt) -> crate::PrismaResult<Option<CliCommand>> {
        let subcommand = opts.subcommand.as_ref();
        let subcommand = match subcommand {
            Some(cmd) => cmd,
            None => return Ok(None),
        };

        match subcommand {
            Subcommand::Cli(ref cliopts) => match cliopts {
                CliOpt::Dmmf => Ok(Some(CliCommand::Dmmf(DmmfRequest {
                    datamodel: opts.datamodel()?,
                    enable_raw_queries: opts.enable_raw_queries,
                    config: opts.configuration(true)?,
                }))),
                CliOpt::GetConfig(input) => Ok(Some(CliCommand::GetConfig(GetConfigRequest {
                    config: opts.configuration(input.ignore_env_var_errors)?,
                    ignore_env_var_errors: input.ignore_env_var_errors,
                }))),
                CliOpt::ExecuteRequest(input) => Ok(Some(CliCommand::ExecuteRequest(ExecuteRequest {
                    query: input.query.clone(),
                    enable_raw_queries: opts.enable_raw_queries,
                    datamodel: opts.datamodel()?,
                    config: opts.configuration(false)?,
                }))),
                CliOpt::DebugPanic(input) => Ok(Some(CliCommand::DebugPanic(DebugPanicRequest {
                    message: input.message.clone(),
                }))),
            },
        }
    }

    pub async fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(request) => Self::dmmf(request).await,
            CliCommand::GetConfig(input) => Self::get_config(input),
            CliCommand::ExecuteRequest(request) => Self::execute_request(request).await,
            CliCommand::DebugPanic(request) => {
                if let Some(message) = request.message {
                    panic!("{}", message);
                } else {
                    panic!("query-engine debug panic");
                }
            }
        }
    }

    async fn dmmf(request: DmmfRequest) -> PrismaResult<()> {
        let datasource = request.config.datasources.first();
        let connector = datasource
            .map(|ds| ds.active_connector)
            .unwrap_or(&psl::datamodel_connector::EmptyDatamodelConnector);
        let referential_integrity = datasource.map(|ds| ds.referential_integrity()).unwrap_or_default();

        // temporary code duplication
        let internal_data_model = InternalDataModelBuilder::from(&request.datamodel).build("".into());
        let query_schema: QuerySchemaRef = Arc::new(schema_builder::build(
            internal_data_model,
            request.enable_raw_queries,
            connector,
            request.config.preview_features().iter().collect(),
            referential_integrity,
        ));

        let dmmf = dmmf::render_dmmf(&request.datamodel, query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{}", serialized);

        Ok(())
    }

    fn get_config(mut req: GetConfigRequest) -> PrismaResult<()> {
        let config = &mut req.config;

        if !req.ignore_env_var_errors {
            config.resolve_datasource_urls_from_env(&[], |key| env::var(key).ok())?;
        }

        let json = psl::mcf::config_to_mcf_json_value(config);
        let serialized = serde_json::to_string(&json)?;

        println!("{}", serialized);

        Ok(())
    }

    async fn execute_request(request: ExecuteRequest) -> PrismaResult<()> {
        let decoded = base64::decode(&request.query)?;
        let decoded_request = String::from_utf8(decoded)?;

        request.config.validate_that_one_datasource_is_provided()?;

        let cx = PrismaContext::builder(request.config, request.datamodel)
            .enable_raw_queries(request.enable_raw_queries)
            .build()
            .await?;

        let cx = Arc::new(cx);

        let handler = GraphQlHandler::new(&*cx.executor, cx.query_schema());
        let res = handler
            .handle(serde_json::from_str(&decoded_request)?, None, None)
            .await;
        let res = serde_json::to_string(&res).unwrap();

        let encoded_response = base64::encode(&res);
        println!("Response: {}", encoded_response); // reason for prefix is explained in TestServer.scala

        Ok(())
    }
}

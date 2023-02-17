use crate::{
    context::{PrismaContext, ServerConfig},
    opt::{CliOpt, PrismaOpt, Subcommand},
    PrismaResult,
};
use query_core::{protocol::EngineProtocol, schema::QuerySchemaRef, schema_builder};
use request_handlers::{dmmf, RequestBody, RequestHandler};
use std::{env, sync::Arc};

pub struct ExecuteRequest {
    query: String,
    schema: psl::ValidatedSchema,
    enable_raw_queries: bool,
    engine_protocol: EngineProtocol,
}

pub struct DmmfRequest {
    schema: psl::ValidatedSchema,
    enable_raw_queries: bool,
}

pub struct GetConfigRequest {
    config: psl::Configuration,
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
                    schema: opts.schema(true)?,
                    enable_raw_queries: opts.enable_raw_queries,
                }))),
                CliOpt::GetConfig(input) => Ok(Some(CliCommand::GetConfig(GetConfigRequest {
                    config: opts.configuration(input.ignore_env_var_errors)?,
                    ignore_env_var_errors: input.ignore_env_var_errors,
                }))),
                CliOpt::ExecuteRequest(input) => {
                    let schema = opts.schema(false)?;
                    let features = schema.configuration.preview_features();

                    Ok(Some(CliCommand::ExecuteRequest(ExecuteRequest {
                        query: input.query.clone(),
                        enable_raw_queries: opts.enable_raw_queries,
                        schema,
                        engine_protocol: opts.engine_protocol(features),
                    })))
                }
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
        let internal_data_model = prisma_models::convert(Arc::new(request.schema));
        let query_schema: QuerySchemaRef =
            Arc::new(schema_builder::build(internal_data_model, request.enable_raw_queries));
        let dmmf = dmmf::render_dmmf(query_schema);
        let serialized = serde_json::to_string_pretty(&dmmf)?;

        println!("{serialized}");

        Ok(())
    }

    fn get_config(mut req: GetConfigRequest) -> PrismaResult<()> {
        let config = &mut req.config;

        config.resolve_datasource_urls_query_engine(&[], |key| env::var(key).ok(), req.ignore_env_var_errors)?;

        let json = psl::get_config::config_to_mcf_json_value(config);
        let serialized = serde_json::to_string(&json)?;

        println!("{serialized}");

        Ok(())
    }

    async fn execute_request(request: ExecuteRequest) -> PrismaResult<()> {
        let decoded = base64::decode(&request.query)?;
        let decoded_request = String::from_utf8(decoded)?;

        request
            .schema
            .configuration
            .validate_that_one_datasource_is_provided()?;

        let mut sc = ServerConfig::default();
        sc.enable_raw_queries = request.enable_raw_queries;
        let cx = PrismaContext::new(request.schema, request.engine_protocol, sc, None).await?;

        let cx = Arc::new(cx);

        let handler = RequestHandler::new(cx.executor(), cx.query_schema(), cx.engine_protocol());

        let body = RequestBody::try_from_str(&decoded_request, cx.engine_protocol())?;

        let res = handler.handle(body, None, None).await;
        let res = serde_json::to_string(&res).unwrap();

        let encoded_response = base64::encode(res);
        println!("Response: {encoded_response}"); // reason for prefix is explained in TestServer.scala

        Ok(())
    }
}

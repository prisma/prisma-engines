use crate::{error::PrismaError, PrismaResult};
use psl::{parser_database::Files, SourceFile};
use query_core::protocol::EngineProtocol;
use serde::Deserialize;
use std::{env, ffi::OsStr, fs::File, io::Read, sync::Arc};
use structopt::StructOpt;

#[derive(Debug, StructOpt, Clone)]
pub enum Subcommand {
    /// Doesn't start a server, but allows running specific commands against Prisma.
    Cli(CliOpt),
}

#[derive(Debug, Clone, StructOpt)]
pub struct ExecuteRequestInput {
    /// GraphQL query to execute
    pub query: String,
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(rename_all = "camelCase")]
pub struct GetConfigInput {
    #[structopt(long)]
    pub ignore_env_var_errors: bool,
}

#[derive(Debug, Clone, StructOpt)]
#[structopt(rename_all = "camelCase")]
pub struct DebugPanicInput {
    #[structopt(long)]
    pub message: Option<String>,
}

#[derive(Debug, StructOpt, Clone)]
pub enum CliOpt {
    /// Output the DMMF from the loaded data model.
    Dmmf,
    /// Get the configuration from the given data model.
    GetConfig(GetConfigInput),
    /// Executes one request and then terminates.
    ExecuteRequest(ExecuteRequestInput),
    /// Artificially panic (for testing the CLI) with an optional message.
    DebugPanic(DebugPanicInput),
}

#[derive(Debug, StructOpt, Clone)]
#[structopt(version = env!("GIT_HASH"))]
pub struct PrismaOpt {
    /// The hostname or IP the query engine should bind to.
    #[structopt(long, short = "H", default_value = "127.0.0.1")]
    pub host: String,

    /// The port the query engine should bind to.
    // NOTE: this is mutually exclusive with path
    #[structopt(long, short, default_value = "4466")]
    pub port: u16,

    /// The unix socket path to listen on
    // NOTE: this is mutually exclusive with port.
    #[structopt(long, short, env)]
    pub unix_path: Option<String>,

    /// Path to the Prisma datamodel file
    #[structopt(long, env = "PRISMA_DML_PATH", parse(from_os_str = load_datamodel_file))]
    pub datamodel_path: Option<String>,

    /// Base64 encoded Prisma datamodel
    #[structopt(long, env = "PRISMA_DML", parse(try_from_str = parse_base64_string))]
    pub datamodel: Option<String>,

    /// Base64 encoded datasource urls, overwriting the ones in the schema
    #[structopt(long, env = "OVERWRITE_DATASOURCES", parse(try_from_str = parse_base64_string))]
    pub overwrite_datasources: Option<String>,

    /// Enables raw SQL queries with executeRaw/queryRaw mutation
    #[structopt(long, short = "r")]
    pub enable_raw_queries: bool,

    /// Enables the GraphQL playground
    #[structopt(long, short = "g")]
    pub enable_playground: bool,

    /// Enables server debug features.
    #[structopt(long = "debug", short = "d")]
    pub enable_debug_mode: bool,

    /// Enables the metrics endpoints
    #[structopt(long, short = "m")]
    pub enable_metrics: bool,

    // Enable the metrics without having to enable the feature in the
    // schema. Used by data proxy to count interactive transactions.
    #[structopt(long)]
    pub dataproxy_metric_override: bool,

    /// Enable query logging [env: LOG_QUERIES=y]
    #[structopt(long, short = "o")]
    pub log_queries: bool,

    /// Set the log format.
    #[structopt(long = "log-format", env = "RUST_LOG_FORMAT")]
    pub log_format: Option<String>,

    /// Enable OpenTelemetry streaming from requests.
    #[structopt(long)]
    pub enable_open_telemetry: bool,

    #[structopt(long)]
    /// Enable tracer to capture logs and traces and return in the response
    pub enable_telemetry_in_response: bool,

    /// The url to the OpenTelemetry collector.
    /// Enabling this will send the OpenTelemtry tracing to a collector
    /// and not via our custom stdout tracer
    #[structopt(long, default_value)]
    pub open_telemetry_endpoint: String,

    /// The protocol the Query Engine will used. Affects mostly the request and response format.
    #[structopt(long, env = "PRISMA_ENGINE_PROTOCOL")]
    pub engine_protocol: Option<String>,

    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
}

#[derive(Debug, Deserialize)]
struct SourceOverride {
    name: String,
    url: String,
}

impl PrismaOpt {
    fn datamodel_str(&self) -> PrismaResult<&str> {
        let res = self
            .datamodel
            .as_deref()
            .or(self.datamodel_path.as_deref())
            .ok_or_else(|| {
                PrismaError::ConfigurationError(
                    "Datamodel should be provided either as path or base64-encoded string.".into(),
                )
            })?;

        Ok(res)
    }

    pub(crate) fn schema(&self, ignore_env_errors: bool) -> PrismaResult<psl::ValidatedSchema> {
        let datamodel_str = self.datamodel_str()?;
        let mut schema = psl::validate(datamodel_str.into());

        schema
            .diagnostics
            .to_result()
            .map_err(|errors| PrismaError::ConversionError(errors, datamodel_str.to_string()))?;

        let datasource_url_overrides: Vec<(String, String)> = if let Some(ref json) = self.overwrite_datasources {
            let datasource_url_overrides: Vec<SourceOverride> = serde_json::from_str(json)?;
            datasource_url_overrides.into_iter().map(|x| (x.name, x.url)).collect()
        } else {
            Vec::new()
        };

        schema
            .configuration
            .resolve_datasource_urls_query_engine(
                &datasource_url_overrides,
                |key| env::var(key).ok(),
                ignore_env_errors,
            )
            .map_err(|errors| PrismaError::ConversionError(errors, datamodel_str.to_string()))?;

        Ok(schema)
    }

    pub(crate) fn configuration(&self, ignore_env_errors: bool) -> PrismaResult<(Files, psl::Configuration)> {
        let datamodel_str = self.datamodel_str()?;
        let source_file = SourceFile::new_allocated(Arc::from(datamodel_str.to_owned().into_boxed_str()));

        let datasource_url_overrides: Vec<(String, String)> = if let Some(ref json) = self.overwrite_datasources {
            let datasource_url_overrides: Vec<SourceOverride> = serde_json::from_str(json)?;
            datasource_url_overrides.into_iter().map(|x| (x.name, x.url)).collect()
        } else {
            Vec::new()
        };

        let file_name = self
            .datamodel_path
            .as_ref()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| "schema.prisma".to_owned());

        let (files, mut config) = psl::parse_configuration_multi_file(&[(file_name, source_file)])
            .map_err(|(_, errors)| PrismaError::ConversionError(errors, datamodel_str.to_string()))?;

        config.resolve_datasource_urls_query_engine(
            &datasource_url_overrides,
            |key| env::var(key).ok(),
            ignore_env_errors,
        )?;
        Ok((files, config))
    }

    /// Extract the log format from on the RUST_LOG_FORMAT env var.
    pub fn log_format(&self) -> crate::LogFormat {
        match self.log_format.as_deref() {
            Some("devel") => crate::LogFormat::Text,
            _ => crate::LogFormat::Json,
        }
    }

    /// Enable query logging
    pub(crate) fn log_queries(&self) -> bool {
        std::env::var("LOG_QUERIES").map(|_| true).unwrap_or(self.log_queries)
    }

    /// The EngineProtocol to use for communication, it will be [EngineProtocol::Json] by
    /// default
    ///
    /// This protocol will determine how the body of an HTTP request made by the client is processed.
    /// [request_handlers::JsonBody] and [request_handlers::GraphqlBody] are in charge
    /// of converting the respective representations into a protocol-agnostic(*)
    /// [query_core::QueryDocument]
    ///
    /// (*) FIXME: at the time of writing, the heuristics to validate the [query_core::QueryDocument]
    /// and  transform it into a [query_core::ParsedObject] require to know which protocol was used
    /// for submitting the query, this is due to the fact that DMMF is no longer used by the client
    /// to understand which types certain values are. See [query_core::QueryDocumentParser]
    ///
    pub(crate) fn engine_protocol(&self) -> EngineProtocol {
        self.engine_protocol
            .as_ref()
            .map(EngineProtocol::from)
            .unwrap_or_else(|| {
                if self.enable_playground {
                    EngineProtocol::Graphql
                } else {
                    EngineProtocol::Json
                }
            })
    }
}

fn parse_base64_string(s: &str) -> PrismaResult<String> {
    match base64::decode(s) {
        Ok(bytes) => String::from_utf8(bytes).map_err(|e| {
            trace!("Error decoding {} from Base64 (invalid UTF-8): {:?}", s, e);

            PrismaError::ConfigurationError("Invalid Base64".into())
        }),
        Err(e) => {
            trace!("Decoding Base64 failed (might not be encoded): {:?}", e);
            Ok(String::from(s))
        }
    }
}

fn load_datamodel_file(path: &OsStr) -> String {
    let mut f = File::open(path).unwrap_or_else(|_| panic!("Could not open datamodel file {path:?}"));
    let mut datamodel = String::new();

    f.read_to_string(&mut datamodel)
        .unwrap_or_else(|_| panic!("Could not read datamodel file: {path:?}"));

    datamodel
}

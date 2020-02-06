use std::{fs::File, io::Read, sync::Arc};

use clap::ArgMatches;
use graphql_parser as gql;
use serde::Deserialize;

use datamodel::json::dmmf::Datamodel;
use query_core::{
    response_ir,
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, CoreError, QuerySchemaBuilder, Responses,
};

use crate::context::PrismaContext;
use crate::error::PrismaError;
use crate::request_handlers::graphql::*;
use crate::{
    data_model_loader::{load_configuration, load_data_model_components},
    dmmf, PrismaResult,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DmmfToDmlInput {
    pub dmmf: Datamodel,
    pub config: serde_json::Value,
}

pub enum CliCommand {
    Dmmf(BuildMode),
    DmmfToDml(DmmfToDmlInput),
    GetConfig(String),
    ExecuteRequest {
        query: String,
        force_transactions: bool,
    },
}

impl CliCommand {
    pub fn new(matches: &ArgMatches, force_transactions: bool) -> Option<Self> {
        if matches.is_present("dmmf") {
            let build_mode = if matches.is_present("legacy") {
                BuildMode::Legacy
            } else {
                BuildMode::Modern
            };

            Some(CliCommand::Dmmf(build_mode))
        } else if matches.is_present("dmmf_to_dml") {
            let path = matches.value_of("dmmf_to_dml").unwrap();
            let file = File::open(path).expect("File should open read only");
            let input: DmmfToDmlInput = serde_json::from_reader(file).expect("File should be proper JSON");
            Some(CliCommand::DmmfToDml(input))
        } else if matches.is_present("get_config") {
            let path = matches.value_of("get_config").unwrap();
            let mut file = File::open(path).expect("File should open read only");

            let mut datamodel = String::new();
            file.read_to_string(&mut datamodel).expect("Couldn't read file");

            Some(CliCommand::GetConfig(datamodel))
        } else if matches.is_present("execute_request") {
            let request = matches.value_of("execute_request").unwrap();

            Some(CliCommand::ExecuteRequest {
                query: request.to_string(),
                force_transactions,
            })
        } else {
            None
        }
    }

    pub fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(build_mode) => Self::dmmf(build_mode),
            CliCommand::DmmfToDml(input) => Self::dmmf_to_dml(input),
            CliCommand::GetConfig(input) => Self::get_config(input),
            CliCommand::ExecuteRequest { query, force_transactions } =>
                Self::execute_request(query, force_transactions),
        }
    }

    fn dmmf(build_mode: BuildMode) -> PrismaResult<()> {
        let (v2components, template) = load_data_model_components()?;

        // temporary code duplication
        let internal_data_model = template.build("".into());
        let capabilities = SupportedCapabilities::empty();

        let schema_builder = QuerySchemaBuilder::new(&internal_data_model, &capabilities, build_mode);
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

    fn execute_request(input: String, force_transactions: bool) -> PrismaResult<()> {
        use futures::executor::block_on;
        use futures::FutureExt;
        use std::panic::AssertUnwindSafe;
        use user_facing_errors::Error;

        let decoded = base64::decode(&input)?;
        let decoded_request = String::from_utf8(decoded)?;
        let cmd = CliCommand::handle_gql_request(decoded_request, force_transactions);

        let response = match block_on(AssertUnwindSafe(cmd).catch_unwind())
        {
            Ok(Ok(responses)) => responses,
            Ok(Err(err)) => {
                let mut responses = response_ir::Responses::default();
                responses.insert_error(err);
                responses
            }
            // panicked
            Err(err) => {
                let mut responses = response_ir::Responses::default();
                let error = Error::from_panic_payload(&err);

                responses.insert_error(error);
                responses
            }
        };

        let response = serde_json::to_string(&response).unwrap();

        let encoded_response = base64::encode(&response);
        println!("Response: {}", encoded_response); // reason for prefix is explained in TestServer.scala

        Ok(())
    }

    async fn handle_gql_request(input: String, force_transactions: bool) -> Result<Responses, PrismaError> {
        let ctx = PrismaContext::builder()
            .legacy(true)
            .force_transactions(force_transactions)
            .build().await?;

        let gql_doc = gql::parse_query(&input)?;
        let query_doc = GraphQLProtocolAdapter::convert(gql_doc, None)?;

        ctx.executor
            .execute(query_doc, Arc::clone(ctx.query_schema()))
            .await
            .map_err(|err| {
                debug!("{}", err);
                let ce: CoreError = err.into();
                ce.into()
            })
    }
}

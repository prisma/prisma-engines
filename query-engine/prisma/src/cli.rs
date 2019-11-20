use crate::{
    data_model_loader::{load_configuration, load_data_model_components},
    dmmf, PrismaResult,
};
use clap::ArgMatches;
use datamodel::json::dmmf::Datamodel;
use query_core::{
    schema::{QuerySchemaRef, SupportedCapabilities},
    BuildMode, QuerySchemaBuilder,
};
use serde::Deserialize;
use std::{fs::File, io::Read, sync::Arc};

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
}

impl CliCommand {
    pub fn new(matches: &ArgMatches) -> Option<Self> {
        if matches.is_present("dmmf") {
            let build_mode = if matches.is_present("legacy") {
                BuildMode::Legacy
            } else {
                BuildMode::Modern
            };

            Some(Self::Dmmf(build_mode))
        } else if matches.is_present("dmmf_to_dml") {
            let path = matches.value_of("dmmf_to_dml").unwrap();
            let file = File::open(path).expect("File should open read only");
            let input: DmmfToDmlInput = serde_json::from_reader(file).expect("File should be proper JSON");
            Some(Self::DmmfToDml(input))
        } else if matches.is_present("get_config") {
            let path = matches.value_of("get_config").unwrap();
            let mut file = File::open(path).expect("File should open read only");

            let mut datamodel = String::new();
            file.read_to_string(&mut datamodel).expect("Couldn't read file");

            Some(Self::GetConfig(datamodel))
        } else {
            None
        }
    }

    pub fn execute(self) -> PrismaResult<()> {
        match self {
            CliCommand::Dmmf(build_mode) => Self::dmmf(build_mode),
            CliCommand::DmmfToDml(input) => Self::dmmf_to_dml(input),
            CliCommand::GetConfig(input) => Self::get_config(input),
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
}

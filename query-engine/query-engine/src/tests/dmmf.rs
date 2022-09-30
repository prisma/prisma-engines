use crate::{
    cli::CliCommand,
    opt::{CliOpt, PrismaOpt, Subcommand},
    PrismaResult,
};
use query_core::{schema::QuerySchema, schema_builder};
use serial_test::serial;
use std::sync::Arc;

pub fn get_query_schema(datamodel_string: &str) -> (QuerySchema, psl::dml::Datamodel) {
    let config = psl::parse_configuration(datamodel_string).unwrap();
    let dm = psl::parse_schema(datamodel_string).unwrap();
    let datasource = config.datasources.first();

    let connector = datasource
        .map(|ds| ds.active_connector)
        .unwrap_or(&psl::datamodel_connector::EmptyDatamodelConnector);
    let relation_mode = datasource.map(|ds| ds.relation_mode()).unwrap_or_default();

    let internal_ref = prisma_models::convert(&dm, "db".to_owned());
    let schema = schema_builder::build(
        internal_ref,
        false,
        connector,
        config.preview_features().iter().collect(),
        relation_mode,
    );

    (schema, psl::lift(&dm))
}

// Tests in this file run serially because the function `get_query_schema` depends on setting an env var.

#[test]
#[serial]
fn must_not_fail_on_missing_env_vars_in_a_datasource() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = env("MISSING_ENV_VAR")
        }

        model Blog {
            blogId String @id
        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = request_handlers::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let inputs = &dmmf.schema.input_object_types;

    assert!(!inputs.is_empty());
}

#[test]
#[serial]
fn must_not_fail_if_no_datasource_is_defined() {
    let schema = r#"
        model Blog {
            blogId String @id
        }
    "#;

    test_dmmf_cli_command(schema).unwrap();
}

#[test]
#[serial]
fn must_not_fail_if_an_invalid_datasource_url_is_provided() {
    let schema = r#"
        datasource pg {
            provider = "postgresql"
            url      = "mysql:://"
        }

        model Blog {
            blogId String @id
        }
    "#;

    test_dmmf_cli_command(schema).unwrap();
}

#[test]
#[serial]
fn must_fail_if_the_schema_is_invalid() {
    let schema = r#"
        // invalid because of field type
        model Blog {
            blogId StringyString @id
        }
    "#;

    assert!(test_dmmf_cli_command(schema).is_err());
}

fn test_dmmf_cli_command(schema: &str) -> PrismaResult<()> {
    let prisma_opt = PrismaOpt {
        host: "".to_string(),
        datamodel: Some(schema.to_string()),
        datamodel_path: None,
        enable_debug_mode: false,
        enable_raw_queries: false,
        enable_playground: false,
        enable_metrics: true,
        log_format: None,
        log_queries: true,
        overwrite_datasources: None,
        port: 123,
        unix_path: None,
        subcommand: Some(Subcommand::Cli(CliOpt::Dmmf)),
        enable_open_telemetry: false,
        open_telemetry_endpoint: String::new(),
    };

    let cli_cmd = CliCommand::from_opt(&prisma_opt)?.unwrap();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let result = runtime.block_on(cli_cmd.execute());
    result?;

    Ok(())
}

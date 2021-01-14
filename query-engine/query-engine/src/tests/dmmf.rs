use crate::{
    cli::CliCommand,
    dmmf::{
        schema::{DmmfOutputField, DmmfOutputType, TypeLocation},
        DataModelMetaFormat,
    },
    opt::{CliOpt, PrismaOpt, Subcommand},
    PrismaResult,
};
use datamodel_connector::ConnectorCapabilities;
use prisma_models::DatamodelConverter;
use query_core::{schema_builder, BuildMode, QuerySchema};
use serial_test::serial;
use std::sync::Arc;

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
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
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

#[test]
#[serial]
fn nullable_fields_should_be_nullable_in_group_by_output_types() {
    let dm = r#"
        datasource pg {
            provider = "postgresql"
            url = "postgresql://"
        }

        model Blog {
            blogId String @id
            firstName   String?
            lastName    String
            age    Int?

        }
    "#;
    let (query_schema, datamodel) = get_query_schema(dm);
    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));

    fn find_output_type<'a>(dmmf: &'a DataModelMetaFormat, type_name: &str) -> &'a DmmfOutputType {
        dmmf.schema
            .output_object_types
            .get("prisma")
            .expect("should exist")
            .into_iter()
            .find(|o| o.name == type_name)
            .expect("should exist")
    }
    fn recursively_assert_fields(dmmf: &DataModelMetaFormat, fields: &Vec<DmmfOutputField>, in_aggregation_type: bool) {
        for field in fields {
            match field.output_type.location {
                TypeLocation::OutputObjectTypes => {
                    let output_type = find_output_type(dmmf, field.output_type.typ.as_str());
                    recursively_assert_fields(dmmf, &output_type.fields, true);
                }
                TypeLocation::Scalar => match (in_aggregation_type, field.name.as_str()) {
                    (false, "blogId") => assert_eq!(field.is_nullable, false),
                    (false, "firstName") => assert_eq!(field.is_nullable, true),
                    (false, "lastName") => assert_eq!(field.is_nullable, false),
                    (false, "age") => assert_eq!(field.is_nullable, true),

                    (true, "blogId") => assert_eq!(field.is_nullable, true),
                    (true, "firstName") => assert_eq!(field.is_nullable, true),
                    (true, "lastName") => assert_eq!(field.is_nullable, true),
                    (true, "age") => assert_eq!(field.is_nullable, true),

                    _ => (),
                },
                _ => (),
            }
        }
    }

    let group_by_output_type = find_output_type(&dmmf, "BlogGroupByOutputType");
    recursively_assert_fields(&dmmf, &group_by_output_type.fields, false)
}

fn test_dmmf_cli_command(schema: &str) -> PrismaResult<()> {
    feature_flags::initialize(&[String::from("all")]).unwrap();

    let prisma_opt = PrismaOpt {
        host: "".to_string(),
        datamodel: Some(schema.to_string()),
        datamodel_path: None,
        enable_debug_mode: false,
        enable_raw_queries: false,
        enable_playground: false,
        legacy: false,
        log_format: None,
        overwrite_datasources: None,
        port: 123,
        raw_feature_flags: vec![],
        unix_path: None,
        subcommand: Some(Subcommand::Cli(CliOpt::Dmmf)),
    };

    let cli_cmd = CliCommand::from_opt(&prisma_opt)?.unwrap();

    let result = test_setup::runtime::run_with_tokio(cli_cmd.execute());
    result?;

    Ok(())
}

fn get_query_schema(datamodel_string: &str) -> (QuerySchema, datamodel::dml::Datamodel) {
    feature_flags::initialize(&[String::from("all")]).unwrap();

    let dm = datamodel::parse_datamodel_and_ignore_datasource_urls(datamodel_string)
        .unwrap()
        .subject;
    let config = datamodel::parse_configuration_and_ignore_datasource_urls(datamodel_string).unwrap();
    let capabilities = match config.subject.datasources.first() {
        Some(ds) => ds.capabilities(),
        None => ConnectorCapabilities::empty(),
    };
    let internal_dm_template = DatamodelConverter::convert(&dm);
    let internal_ref = internal_dm_template.build("db".to_owned());

    (
        schema_builder::build(internal_ref, BuildMode::Modern, false, capabilities),
        dm,
    )
}

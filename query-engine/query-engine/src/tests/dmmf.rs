use datamodel::transform::ast_to_dml::reserved_model_names::TypeNameValidator;
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

    let inputs = &dmmf.schema.input_types;

    inputs
        .iter()
        .find(|input| input.name == "BlogCreateInput")
        .expect("finding BlogCreateInput");
}

#[test]
#[serial]
fn list_of_reserved_model_names_must_be_up_to_date() {
    let dm = r#"
        datasource mydb {
           provider       = "postgresql"
           url            = "postgresql://localhost"
        }

        model Blog {
            id          Int @id
            intReq      Int
            intOpt      Int?
            flaotReq    Float
            flaotOpt    Float?
            boolReq     Boolean
            boolOpt     Boolean?
            stringReq   String
            stringOpt   String?
            datetimeReq DateTime
            datetimeOpt DateTime?
            jsonReq     Json
            jsonOpt     Json?

            posts       Post[]
        }

        model Post {
          id     Int @id
          blogId Int

          blog   Blog @relation(fields: blogId, references: id)
        }
    "#;

    let (query_schema, datamodel) = get_query_schema(dm);

    let dmmf = crate::dmmf::render_dmmf(&datamodel, Arc::new(query_schema));
    let inputs = &dmmf.schema.input_types;
    let model_names: Vec<_> = datamodel.models.iter().map(|m| m.name.as_str()).collect();

    let validator = TypeNameValidator::new();

    let mut types_that_should_be_reserved: Vec<String> = Vec::new();
    types_that_should_be_reserved.append(&mut dmmf.schema.enums.iter().map(|en| en.name.clone()).collect());
    types_that_should_be_reserved.append(&mut inputs.iter().map(|input| input.name.clone()).collect());

    types_that_should_be_reserved = types_that_should_be_reserved
        .into_iter()
        // this filters out dynamic types names like e.g. `BlogCreateInput` that are not part of the static deny list
        .filter(|type_name| !model_names.iter().any(|name| type_name.contains(name)))
        .filter(|type_name| !validator.is_reserved(&type_name))
        .collect();

    if !types_that_should_be_reserved.is_empty() {
        panic!(
            "Some type names are not part of the reserved model names but they should be!\n{}",
            types_that_should_be_reserved.join(",\n")
        )
    }
}

fn get_query_schema(datamodel_string: &str) -> (QuerySchema, datamodel::dml::Datamodel) {
    feature_flags::initialize(&vec![String::from("all")]).unwrap();

    let dm = datamodel::parse_datamodel_and_ignore_datasource_urls(datamodel_string).unwrap();
    let config = datamodel::parse_configuration_and_ignore_datasource_urls(datamodel_string).unwrap();
    let capabilities = match config.datasources.first() {
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

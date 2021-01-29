use crate::dmmf::{schema::*, DataModelMetaFormat};
use datamodel_connector::ConnectorCapabilities;
use prisma_models::DatamodelConverter;
use query_core::{schema_builder, BuildMode, QuerySchema};

pub fn get_query_schema(datamodel_string: &str) -> (QuerySchema, datamodel::dml::Datamodel) {
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

pub fn find_output_type<'a>(dmmf: &'a DataModelMetaFormat, namespace: &str, type_name: &str) -> &'a DmmfOutputType {
    dmmf.schema
        .output_object_types
        .get(namespace)
        .unwrap_or_else(|| panic!("unknown dmmf namespace {}", namespace))
        .iter()
        .find(|o| o.name == type_name)
        .unwrap_or_else(|| panic!("could not find output type named {}", type_name))
}

pub fn find_input_type<'a>(dmmf: &'a DataModelMetaFormat, namespace: &str, type_name: &str) -> &'a DmmfInputType {
    dmmf.schema
        .input_object_types
        .get(namespace)
        .unwrap_or_else(|| panic!("unknown dmmf namespace {}", namespace))
        .iter()
        .find(|o| o.name == type_name)
        .unwrap_or_else(|| panic!("could not find output type named {}", type_name))
}

pub fn iterate_output_type_fields<P>(output_type: &DmmfOutputType, dmmf: &DataModelMetaFormat, iteratee: &P)
where
    P: Fn(&DmmfOutputField, &DmmfOutputType),
{
    for field in &output_type.fields {
        match field.output_type.location {
            TypeLocation::OutputObjectTypes => {
                let namespace = field
                    .output_type
                    .namespace
                    .as_ref()
                    .expect("a namespace is required to iterate over a nested output type but could not find one");
                let nested_output_type = find_output_type(dmmf, namespace, field.output_type.typ.as_str());

                iteratee(&field, nested_output_type);
                iterate_output_type_fields(nested_output_type, dmmf, iteratee)
            }
            TypeLocation::Scalar | TypeLocation::EnumTypes => {
                iteratee(&field, output_type);
            }
            _ => (),
        }
    }
}

pub fn iterate_input_type_fields<P>(input_type: &DmmfInputType, dmmf: &DataModelMetaFormat, iteratee: &P)
where
    P: Fn(&DmmfTypeReference, &DmmfInputField, &DmmfInputType),
{
    for field in &input_type.fields {
        for input_type_ref in &field.input_types {
            match input_type_ref.location {
                TypeLocation::InputObjectTypes => {
                    let namespace = input_type_ref
                        .namespace
                        .as_ref()
                        .expect("a namespace is required to iterate over a nested output type but could not find one");
                    let nested_input_type = find_input_type(dmmf, namespace, input_type_ref.typ.as_str());

                    iteratee(&input_type_ref, &field, nested_input_type);
                    iterate_input_type_fields(nested_input_type, dmmf, iteratee)
                }
                TypeLocation::Scalar | TypeLocation::EnumTypes => {
                    iteratee(&input_type_ref, &field, input_type);
                }
                _ => (),
            }
        }
    }
}

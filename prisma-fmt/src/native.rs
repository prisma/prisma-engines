use psl::datamodel_connector::NativeTypeConstructor;

use crate::schema_file_input::SchemaFileInput;

pub(crate) fn run(input: &str) -> String {
    let schema: Vec<_> = match serde_json::from_str::<SchemaFileInput>(input) {
        Ok(input) => input.into(),
        Err(_) => return "[]".to_owned(),
    };

    let validated_configuration = match psl::parse_configuration_multi_file(&schema) {
        Ok((_, validated_configuration)) => validated_configuration,
        Err(_) => return "[]".to_owned(),
    };

    if validated_configuration.datasources.len() != 1 {
        return "[]".to_owned();
    }

    let datasource = &validated_configuration.datasources[0];
    let available_native_type_constructors = datasource.active_connector.available_native_type_constructors();
    let available_native_type_constructors: Vec<SerializableNativeTypeConstructor> =
        available_native_type_constructors.iter().map(From::from).collect();

    serde_json::to_string(&available_native_type_constructors).expect("Failed to render JSON")
}

#[derive(serde::Serialize)]
struct SerializableNativeTypeConstructor {
    pub name: &'static str,
    pub _number_of_args: usize,
    pub _number_of_optional_args: usize,
    pub prisma_types: Vec<&'static str>,
}

impl From<&NativeTypeConstructor> for SerializableNativeTypeConstructor {
    fn from(nt: &NativeTypeConstructor) -> Self {
        let NativeTypeConstructor {
            name,
            number_of_args,
            number_of_optional_args,
            prisma_types,
        } = nt;
        SerializableNativeTypeConstructor {
            name,
            _number_of_args: *number_of_args,
            _number_of_optional_args: *number_of_optional_args,
            prisma_types: prisma_types.iter().map(|st| st.as_str()).collect(),
        }
    }
}

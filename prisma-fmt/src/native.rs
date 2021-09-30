pub(crate) fn run(schema: &str) -> String {
    let validated_configuration = match datamodel::parse_configuration(schema) {
        Ok(validated_configuration) => validated_configuration,
        Err(_) => return "[]".to_owned(),
    };

    if validated_configuration.subject.datasources.len() != 1 {
        return "[]".to_owned();
    }

    let datasource = &validated_configuration.subject.datasources[0];
    let available_native_type_constructors = datasource.active_connector.available_native_type_constructors();

    serde_json::to_string(available_native_type_constructors).expect("Failed to render JSON")
}

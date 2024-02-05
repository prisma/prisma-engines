pub(crate) fn run(schema: &str) -> String {
    let datamodel_result = psl::parse_configuration(schema);

    match datamodel_result {
        Ok((validated_configuration, _)) => {
            if validated_configuration.datasources.len() != 1 {
                "[]".to_string()
            } else if let Some(datasource) = validated_configuration.datasources.first() {
                let available_referential_actions = datasource
                    .active_connector
                    .referential_actions()
                    .iter()
                    .map(|act| format!("{act:?}"))
                    .collect::<Vec<_>>();

                serde_json::to_string(&available_referential_actions).expect("Failed to render JSON")
            } else {
                "[]".to_string()
            }
        }
        _ => "[]".to_owned(),
    }
}

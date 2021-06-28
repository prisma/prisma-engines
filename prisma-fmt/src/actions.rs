use std::io::{self, Read};

use datamodel::common::preview_features::PreviewFeature;

pub fn run() {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    let datamodel_result = datamodel::parse_configuration(&datamodel_string);

    match datamodel_result {
        Ok(validated_configuration) => {
            if validated_configuration.subject.datasources.len() != 1 {
                print!("[]")
            } else if let Some(datasource) = validated_configuration.subject.datasources.first() {
                if validated_configuration
                    .subject
                    .preview_features()
                    .any(|f| *f == PreviewFeature::ReferentialActions)
                {
                    let available_referential_actions = datasource
                        .active_connector
                        .referential_actions()
                        .iter()
                        .map(|act| format!("{:?}", act))
                        .collect::<Vec<_>>();

                    let json = serde_json::to_string(&available_referential_actions).expect("Failed to render JSON");

                    print!("{}", json)
                } else {
                    print!("[]")
                }
            } else {
                print!("[]")
            }
        }
        _ => print!("[]"),
    }
}

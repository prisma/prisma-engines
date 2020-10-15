use std::io;
use std::io::Read;

pub fn run() {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    let datamodel_result = datamodel::parse_configuration_and_ignore_datasource_urls(&datamodel_string);

    match datamodel_result {
        Ok(validated_configuration) => {
            if validated_configuration.subject.datasources.len() != 1 {
                print!("[]")
            } else if let Some(datasource) = validated_configuration.subject.datasources.first() {
                let available_native_type_constructors =
                    datasource.active_connector.available_native_type_constructors();

                let json = serde_json::to_string(available_native_type_constructors).expect("Failed to render JSON");

                print!("{}", json)
            } else {
                print!("[]")
            }
        }
        _ => print!("[]"),
    }
}

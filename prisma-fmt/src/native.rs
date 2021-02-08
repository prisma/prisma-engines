use std::io::{self, Read};

use datamodel::{diagnostics::Validator, Configuration};

pub fn run() {
    let mut datamodel_string = String::new();

    io::stdin()
        .read_to_string(&mut datamodel_string)
        .expect("Unable to read from stdin.");

    let validator = Validator::<Configuration>::new();

    match validator.parse_str(&datamodel_string) {
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

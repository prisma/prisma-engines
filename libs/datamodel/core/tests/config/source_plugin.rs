use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};
use pretty_assertions::assert_eq;

const DATAMODEL: &str = r#"
datasource db1 {
    provider = "postgresql"
    url = env("URL_CUSTOM_1")
}

datasource db2 {
    provider = "mysql"
    url = "https://localhost"
}


model User {
    id Int @id
    firstName String @custom_1.mapToInt
    lastName String @custom_1.mapToInt
    email String
}

model Post {
    id Int @id
    likes String @custom_2.mapToInt
    comments Int
}
"#;

#[test]
fn serialize_sources_to_dmmf() {
    std::env::set_var("URL_CUSTOM_1", "https://localhost");
    let config = datamodel::parse_configuration(DATAMODEL).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = r#"[
  {
    "name": "db1",
    "connectorType": "postgresql",
    "url": {
        "fromEnvVar": "URL_CUSTOM_1",
        "value": "https://localhost"       
    }
  },
  {
    "name": "db2",
    "connectorType": "mysql",
    "url": {
        "fromEnvVar": null,
        "value": "https://localhost"      
    }
  }
]"#;

    println!("{}", rendered);

    assert_eq_json(&rendered, expected);
}

#[test]
fn must_forbid_env_functions_in_provider_field() {
    let schema = r#"
        datasource ds {
            provider = env("DB_PROVIDER")
            url = env("DB_URL")
        }
    "#;
    std::env::set_var("DB_PROVIDER", "postgresql");
    std::env::set_var("DB_URL", "https://localhost");
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_functional_evaluation_error(
        "A datasource must not use the env() function in the provider argument.",
        Span::new(9, 108),
    ));
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

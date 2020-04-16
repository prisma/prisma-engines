use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError};
use pretty_assertions::assert_eq;
use serial_test::serial;

const DATAMODEL: &str = r#"
datasource db1 {
    provider = "postgresql"
    url = env("URL_CUSTOM_1")
}

datasource db2 {
    provider = "mysql"
    url = "mysql://localhost"
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
#[serial]
fn serialize_sources_to_dmmf() {
    std::env::set_var("URL_CUSTOM_1", "postgresql://localhost");
    let config = datamodel::parse_configuration(DATAMODEL).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = r#"[
  {
    "name": "db1",
    "connectorType": "postgresql",
    "url": {
        "fromEnvVar": "URL_CUSTOM_1",
        "value": "postgresql://localhost"       
    }
  },
  {
    "name": "db2",
    "connectorType": "mysql",
    "url": {
        "fromEnvVar": null,
        "value": "mysql://localhost"      
    }
  }
]"#;

    println!("{}", rendered);

    assert_eq_json(&rendered, expected);
}

#[test]
#[serial]
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

#[test]
fn must_error_for_empty_urls() {
    let schema = r#"
        datasource myds {
            provider = "sqlite"
            url = ""
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_source_validation_error(
        "You must provide a nonempty URL for the datasource `myds`.",
        "myds",
        Span::new(77, 79),
    ));
}

#[test]
#[serial]
fn must_error_for_empty_urls_derived_from_env_vars() {
    std::env::set_var("DB_URL", "  ");
    let schema = r#"
        datasource myds {
            provider = "sqlite"
            url = env("DB_URL")
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_source_validation_error(
        "You must provide a nonempty URL for the datasource `myds`. The environment variable `DB_URL` resolved to an empty string.",
        "myds",
        Span::new(77, 90),
    ));
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_mysql() {
    let schema = r#"
        datasource myds {
            provider = "mysql"
            url = "postgresql://"
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_source_validation_error(
        "The URL for datasource `myds` must start with the protocol `mysql://`.",
        "myds",
        Span::new(76, 91),
    ));
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_postgresql() {
    let schema = r#"
        datasource myds {
            provider = "postgresql"
            url = "mysql://"
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_source_validation_error(
        "The URL for datasource `myds` must start with the protocol `postgresql://`.",
        "myds",
        Span::new(81, 91),
    ));
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_sqlite() {
    let schema = r#"
        datasource myds {
            provider = "sqlite"
            url = "mysql://"
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_source_validation_error(
        "The URL for datasource `myds` must start with the protocol `sqlite://`.",
        "myds",
        Span::new(77, 87),
    ));
}

#[test]
fn new_lines_in_source_must_work() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = "postgresql://localhost"
        
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = r#"[
        {
          "name": "ds",
          "connectorType": "postgresql",
          "url": {
              "fromEnvVar": null,
              "value": "postgresql://localhost"       
          }
        }
    ]"#;

    println!("{}", rendered);

    assert_eq_json(&rendered, expected);
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

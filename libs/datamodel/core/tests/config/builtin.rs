use crate::common::ErrorAsserts;
use datamodel::error::DatamodelError;
use pretty_assertions::assert_eq;

#[test]
fn serialize_builtin_sources_to_dmmf() {
    std::env::set_var("pg2", "https://localhost/postgres2");
    const DATAMODEL: &str = r#"
        datasource pg1 {
            provider = "postgresql"
            url = "https://localhost/postgres1"
        }
        
        datasource pg2 {
            provider = "postgresql"
            url = env("pg2")
        }
        
        datasource sqlite1 {
            provider = "sqlite"
            url = "https://localhost/sqlite1"
        }
        
        datasource mysql1 {
            provider = "mysql"
            url = "https://localhost/mysql"
        }
    "#;
    let config = datamodel::parse_configuration(DATAMODEL).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = r#"[
  {
    "name": "pg1",
    "connectorType": "postgresql",
    "url": {
      "fromEnvVar": null,
      "value": "https://localhost/postgres1"
    }
  },
  {
    "name": "pg2",
    "connectorType": "postgresql",
    "url": {
      "fromEnvVar": "pg2",
      "value": "https://localhost/postgres2"
    }
  },
  {
    "name": "sqlite1",
    "connectorType": "sqlite",
    "url": {
      "fromEnvVar": null,
      "value": "https://localhost/sqlite1"
    }
  },
  {
    "name": "mysql1",
    "connectorType": "mysql",
    "url": {
      "fromEnvVar": null,
      "value": "https://localhost/mysql"
    }
  }
]"#;

    print!("{}", &rendered);

    assert_eq_json(&rendered, expected);
}

#[test]
fn fail_to_load_sources_for_invalid_source() {
    let invalid_datamodel: &str = r#"
        datasource pg1 {
            provider = "AStrangeHalfMongoDatabase"
            url = "https://localhost/postgres1"
        }
    "#;
    let res = datamodel::parse_configuration(invalid_datamodel);

    if let Err(error) = res {
        error.assert_is(DatamodelError::SourceNotKnownError {
            source_name: String::from("AStrangeHalfMongoDatabase"),
            span: datamodel::ast::Span::new(49, 76),
        });
    } else {
        panic!("Expected error.")
    }
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

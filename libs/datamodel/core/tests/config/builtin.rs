use crate::common::ErrorAsserts;
use datamodel::error::DatamodelError;
use pretty_assertions::assert_eq;

#[test]
fn serialize_builtin_sources_to_dmmf() {
    std::env::set_var("pg2", "postgresql://localhost/postgres2");
    const DATAMODEL: &str = r#"
        datasource pg1 {
            provider = "postgresql"
            url = "postgresql://localhost/postgres1"
        }
        
        datasource pg2 {
            provider = "postgresql"
            url = env("pg2")
        }
        
        datasource sqlite1 {
            provider = "sqlite"
            url = "sqlite://file.db"
        }
        
        datasource mysql1 {
            provider = "mysql"
            url = "mysql://localhost"
        }
    "#;
    let config = datamodel::parse_configuration(DATAMODEL).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = r#"[
  {
    "name": "pg1",
    "provider": ["postgresql"],
    "activeProvider": "postgresql",
    "url": {
      "fromEnvVar": null,
      "value": "postgresql://localhost/postgres1"
    }
  },
  {
    "name": "pg2",
    "provider": ["postgresql"],
    "activeProvider": "postgresql",
    "url": {
      "fromEnvVar": "pg2",
      "value": "postgresql://localhost/postgres2"
    }
  },
  {
    "name": "sqlite1",
    "provider": ["sqlite"],
    "activeProvider": "sqlite",
    "url": {
      "fromEnvVar": null,
      "value": "sqlite://file.db"
    }
  },
  {
    "name": "mysql1",
    "provider": ["mysql"],
    "activeProvider": "mysql",
    "url": {
      "fromEnvVar": null,
      "value": "mysql://localhost"
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
        error.assert_is(DatamodelError::DatasourceProviderNotKnownError {
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

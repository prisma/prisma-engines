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

const INVALID_DATAMODEL: &str = r#"
datasource pg1 {
    provider = "AStrangeHalfMongoDatabase"
    url = "https://localhost/postgres1"
}
"#;

#[test]
fn fail_to_load_sources_for_invalid_source() {
    let res = datamodel::parse_configuration(INVALID_DATAMODEL);

    if let Err(error) = res {
        error.assert_is(DatamodelError::SourceNotKnownError {
            source_name: String::from("AStrangeHalfMongoDatabase"),
            span: datamodel::ast::Span::new(33, 60),
        });
    } else {
        panic!("Expected error.")
    }
}

const ENABLED_DISABLED_SOURCE: &str = r#"
datasource chinook {
  provider = "sqlite"
  url = "file:../db/production.db"
  enabled = true
}

datasource chinook {
  provider = "sqlite"
  url = "file:../db/staging.db"
  enabled = false
}

"#;

#[test]
fn enable_disable_source_through_argument() {
    let config = datamodel::parse_configuration(ENABLED_DISABLED_SOURCE).unwrap();

    assert_eq!(config.datasources.len(), 1);

    let source = &config.datasources[0];

    assert_eq!(source.name(), "chinook");
    assert_eq!(source.connector_type(), "sqlite");
    assert_eq!(source.url().value, "file:../db/production.db");
}

const ENABLED_DISABLED_SOURCE_ENV: &str = r#"
// will be disabled by the env var resolving to false
datasource one {
  provider = "sqlite"
  url = "file:../db/one.db"
  enabled = env("ONE")
}

// will be enabled by the env var resolving to true
datasource two {
  provider = "sqlite"
  url = "file:../db/two.db"
  enabled = env("TWO")
}

// will be enabled by sheer presence of the env var
datasource three {
    provider = "sqlite"
    url = "file:../db/three.db"
    enabled = env("THREE")
}

// will be disabled by sheer presence of the env var
datasource four {
    provider = "sqlite"
    url = "file:../db/four.db"
    enabled = env("FOUR")
}

// will be enabled because normal
datasource five {
    provider = "sqlite"
    url = "file:../db/five.db"
}

"#;

#[test]
fn enable_and_disable_source_through_boolean_env_var() {
    std::env::set_var("ONE", "false");
    std::env::set_var("TWO", "true");
    std::env::set_var("THREE", "FOOBAR");

    let config = datamodel::parse_configuration(ENABLED_DISABLED_SOURCE_ENV).unwrap();

    assert_eq!(config.datasources.len(), 3);

    let source1 = &config.datasources[0];
    assert_eq!(source1.name(), "two");
    assert_eq!(source1.connector_type(), "sqlite");
    assert_eq!(source1.url().value, "file:../db/two.db");

    let source2 = &config.datasources[1];
    assert_eq!(source2.name(), "three");
    assert_eq!(source2.connector_type(), "sqlite");
    assert_eq!(source2.url().value, "file:../db/three.db");

    let source3 = &config.datasources[2];
    assert_eq!(source3.name(), "five");
    assert_eq!(source3.connector_type(), "sqlite");
    assert_eq!(source3.url().value, "file:../db/five.db");
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

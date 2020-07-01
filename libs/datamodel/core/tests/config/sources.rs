use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError, StringFromEnvVar};
use pretty_assertions::assert_eq;
use serial_test::serial;

#[test]
#[serial]
fn serialize_sources_to_dmmf() {
    let dml = r#"
datasource db1 {
    provider = ["sqlite", "postgresql"]
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

    std::env::set_var("URL_CUSTOM_1", "postgresql://localhost");
    let config = datamodel::parse_configuration(dml).unwrap();
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = r#"[
  {
    "name": "db1",
    "provider": ["sqlite", "postgresql"],
    "activeProvider": "postgresql",
    "url": {
        "fromEnvVar": "URL_CUSTOM_1",
        "value": "postgresql://localhost"       
    }
  },
  {
    "name": "db2",
    "provider": ["mysql"],
    "activeProvider": "mysql",
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
#[serial]
fn must_forbid_env_functions_in_provider_field_even_if_missing() {
    let schema = r#"
        datasource ds {
            provider = env("DB_PROVIDER")
            url = env("DB_URL")
        }
    "#;
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
fn must_error_for_empty_provider_arrays() {
    let schema = r#"
        datasource myds {
            provider = []
            url = "postgres://"
        }
    "#;

    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let errors = config.err().expect("This must error");
    errors.assert_is(DatamodelError::new_validation_error(
        "This line is not a valid definition within a datasource.",
        Span::new(39, 53),
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
          "provider": ["postgresql"],
          "activeProvider": "postgresql",
          "url": {
              "fromEnvVar": null,
              "value": "postgresql://localhost"       
          }
        }
    ]"#;

    println!("{}", rendered);

    assert_eq_json(&rendered, expected);
}

#[test]
#[serial]
fn must_error_if_env_var_is_missing() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = env("DATABASE_URL")        
        }
    "#;

    let result = datamodel::parse_configuration(schema);
    assert!(result.is_err());
    let errors = result.err().unwrap();
    errors.assert_is(DatamodelError::new_environment_functional_evaluation_error(
        "DATABASE_URL",
        Span::new(75, 94),
    ));
}

#[test]
#[serial]
fn must_succeed_if_env_var_is_missing_but_override_was_provided() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = env("DATABASE_URL")        
        }
    "#;

    let url = "postgres://localhost";
    let overrides = vec![("ds".to_string(), url.to_string())];
    let config = datamodel::parse_configuration_with_url_overrides(schema, overrides).unwrap();
    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        from_env_var: None,
        value: url.to_string(),
    });
}

#[test]
#[serial]
fn must_succeed_if_env_var_exists_and_override_was_provided() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = env("DATABASE_URL")        
        }
    "#;
    std::env::set_var("DATABASE_URL", "postgres://hostfoo");

    let url = "postgres://hostbar";
    let overrides = vec![("ds".to_string(), url.to_string())];
    let config = datamodel::parse_configuration_with_url_overrides(schema, overrides).unwrap();
    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        from_env_var: None,
        value: url.to_string(),
    });
}

#[test]
#[serial]
fn must_succeed_with_overrides() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = "postgres://hostfoo"     
        }
    "#;

    let url = "postgres://hostbar";
    let overrides = vec![("ds".to_string(), url.to_string())];
    let config = datamodel::parse_configuration_with_url_overrides(schema, overrides).unwrap();
    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        from_env_var: None,
        value: url.to_string(),
    });
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

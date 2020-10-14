use crate::common::*;
use datamodel::{ast::Span, error::DatamodelError, StringFromEnvVar};
use pretty_assertions::assert_eq;
use serial_test::serial;

#[test]
#[serial]
fn must_error_if_multiple_datasources_are_defined() {
    let schema = r#"
datasource db1 {
    provider = "postgresql"
    url = "postgresql://localhost"
}

datasource db2 {
    provider = "mysql"
    url = "mysql://localhost"
}
"#;

    let errors = parse_error(&schema);
    errors.assert_length(2);
    errors.assert_is_at(
        0,
        DatamodelError::new_source_validation_error("You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.", "db1", Span::new(1, 82)),
    );
    errors.assert_is_at(
        1,
        DatamodelError::new_source_validation_error("You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.", "db2", Span::new(84, 155)),
    );
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
        "The URL for datasource `myds` must start with the protocol `file:`.",
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

#[test]
#[serial]
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

#[test]
#[serial]
fn fail_when_no_source_is_declared() {
    let invalid_datamodel: &str = r#"        "#;
    let res = datamodel::parse_configuration(invalid_datamodel).unwrap();

    if let Err(error) = res.validate_that_one_datasource_is_provided() {
        error.assert_is(DatamodelError::ValidationError {
            message: "You defined no datasource. You must define exactly one datasource.".to_string(),
            span: datamodel::ast::Span::new(0, 0),
        });
    } else {
        panic!("Expected error.")
    }
}

#[test]
fn microsoft_sql_server_preview_feature_must_work() {
    let schema = r#"
        datasource redmond {
            provider = "sqlserver"
            url = "sqlserver://localhost:1645;foo=bar"
            previewFeatures = ["microsoftSqlServer"]
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let data_source = config.datasources.first().unwrap();

    assert!(data_source
        .preview_features
        .contains(&String::from("microsoftSqlServer")));
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

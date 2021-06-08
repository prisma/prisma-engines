use crate::common::*;
use datamodel::{ast::Span, common::preview_features::*, diagnostics::DatamodelError, StringFromEnvVar};
use pretty_assertions::assert_eq;

#[test]
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

    let diagnostics = parse_error(&schema);
    diagnostics.assert_length(2);
    diagnostics.assert_is_at(
        0,
        DatamodelError::new_source_validation_error("You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.", "db1", Span::new(1, 82)),
    );
    diagnostics.assert_is_at(
        1,
        DatamodelError::new_source_validation_error("You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.", "db2", Span::new(84, 155)),
    );
}

#[test]
fn must_forbid_env_functions_in_provider_field() {
    let schema = r#"
        datasource ds {
            provider = env("DB_PROVIDER")
            url = env("DB_URL")
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    assert!(config.is_err());
    let diagnostics = config.err().expect("This must error");
    diagnostics.assert_is(DatamodelError::new_functional_evaluation_error(
        "A datasource must not use the env() function in the provider argument.",
        Span::new(9, 108),
    ));
}

#[test]
fn must_forbid_env_functions_in_provider_field_even_if_missing() {
    let schema = r#"
        datasource ds {
            provider = env("DB_PROVIDER")
            url = env("DB_URL")
        }
    "#;
    let config = datamodel::parse_configuration(schema);
    let diagnostics = config.err().expect("This must error");
    diagnostics.assert_is(DatamodelError::new_functional_evaluation_error(
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

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "You must provide a nonempty URL",
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
    let diagnostics = config.err().expect("This must error");
    diagnostics.assert_is(DatamodelError::new_validation_error(
        "This line is not a valid definition within a datasource.",
        Span::new(39, 53),
    ));
}

#[test]
fn must_error_for_empty_urls_derived_from_env_vars() {
    std::env::set_var("DB_URL_EMPTY_0001", "  ");
    let schema = r#"
        datasource myds {
            provider = "sqlite"
            url = env("DB_URL_EMPTY_0001")
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "You must provide a nonempty URL. The environment variable `DB_URL_EMPTY_0001` resolved to an empty string.",
        "myds",
        Span::new(77, 101),
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

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "the URL must start with the protocol `mysql://`.",
        "myds",
        Span::new(76, 91),
    ));
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_mysql_shadow_database_url() {
    let schema = r#"
        datasource myds {
            provider = "mysql"
            url = "mysql://"
            shadowDatabaseUrl = "postgresql://"
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_shadow_database_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "the shadow database URL must start with the protocol `mysql://`.",
        "myds",
        Span::new(119, 134),
    ));
}

#[test]
fn must_not_error_for_empty_shadow_database_urls_derived_from_env_vars() {
    std::env::set_var("EMPTY_SHADOW_DB URL_0129", "  ");

    let schema = r#"
        datasource myds {
            provider = "postgres"
            url = "postgres://"
            shadowDatabaseUrl = env("EMPTY_SHADOW_DB URL_0129")
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let shadow_database_url = config.subject.datasources[0].load_shadow_database_url().unwrap();

    assert!(shadow_database_url.is_none());
}

#[test]
fn must_not_error_for_shadow_database_urls_derived_from_missing_env_vars() {
    let schema = r#"
        datasource myds {
            provider = "postgres"
            url = "postgres://"
            shadowDatabaseUrl = env("SHADOW_DATABASE_URL_NOT_SET_21357")
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let shadow_database_url = config.subject.datasources[0].load_shadow_database_url().unwrap();

    assert!(shadow_database_url.is_none());
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_postgresql() {
    let schema = r#"
        datasource myds {
            provider = "postgresql"
            url = "mysql://"
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "the URL must start with the protocol `postgresql://` or `postgres://`.",
        "myds",
        Span::new(81, 91),
    ));
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_postgresql_shadow_database_url() {
    let schema = r#"
        datasource myds {
            provider = "postgresql"
            url = "postgresql://"
            shadowDatabaseUrl = "mysql://"
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_shadow_database_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "the shadow database URL must start with the protocol `postgresql://` or `postgres://`.",
        "myds",
        Span::new(129, 139),
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

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_source_validation_error(
        "the URL must start with the protocol `file:`.",
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

    let config = parse_configuration(schema);
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
fn must_error_if_env_var_is_missing() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = env("MISSING_DATABASE_URL_0001")
        }
    "#;

    let config = datamodel::parse_configuration(schema).unwrap();
    let diagnostics = config.subject.datasources[0].load_url().unwrap_err();

    diagnostics.assert_is(DatamodelError::new_environment_functional_evaluation_error(
        "MISSING_DATABASE_URL_0001".into(),
        Span::new(75, 107),
    ));
}

#[test]
fn must_succeed_if_env_var_is_missing_but_override_was_provided() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = env("MISSING_DATABASE_URL_0002")
        }
    "#;

    let url = "postgres://localhost";
    let overrides = vec![("ds".to_string(), url.to_string())];
    let mut config = parse_configuration(schema);
    config.resolve_datasource_urls_from_env(&overrides).unwrap();
    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        value: Some(url.to_string()),
        from_env_var: None,
    });
}

#[test]
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
    let mut config = parse_configuration(schema);
    config.resolve_datasource_urls_from_env(&overrides).unwrap();
    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    assert_eq!(data_source.url.value.as_deref(), Some(url));

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DATABASE_URL");
}

#[test]
fn must_succeed_with_overrides() {
    let schema = r#"
        datasource ds {
          provider = "postgresql"
          url = "postgres://hostfoo"
        }
    "#;

    let url = "postgres://hostbar";
    let overrides = &[("ds".to_string(), url.to_string())];
    let mut config = parse_configuration(schema);
    config.resolve_datasource_urls_from_env(overrides).unwrap();

    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    assert_eq!(data_source.url.value.as_deref(), Some(url));
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

    res.err()
        .unwrap()
        .assert_is(DatamodelError::DatasourceProviderNotKnownError {
            provider: String::from("AStrangeHalfMongoDatabase"),
            span: datamodel::ast::Span::new(49, 76),
        });
}

#[test]
fn fail_when_preview_features_are_declared() {
    let dml = r#"
    datasource db {
        provider = "mysql"
        url = "mysql://"
        previewFeatures = ["foo"]
    }
    "#;

    let errors = parse_error(dml);
    errors.assert_is(DatamodelError::new_connector_error(
        "Preview features are only supported in the generator block. Please move this field to the generator block.",
        Span::new(99, 106),
    ));
}

#[test]
fn fail_when_no_source_is_declared() {
    let invalid_datamodel: &str = r#"        "#;
    let res = parse_configuration(invalid_datamodel);

    if let Err(diagnostics) = res.validate_that_one_datasource_is_provided() {
        diagnostics.assert_is(DatamodelError::ValidationError {
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
        }

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["microsoftSqlServer"]
        }

    "#;

    let config = parse_configuration(schema);
    let generator = config.generators.first().unwrap();

    assert!(generator.preview_features.contains(&PreviewFeature::MicrosoftSqlServer));
}

#[test]
fn planet_scale_mode_without_preview_feature_errors() {
    let schema_1 = r#"
    datasource ps {
        provider = "mysql"
        planetScaleMode = true
        url = "mysql://root:prisma@localhost:3306/mydb"
    }
    "#;

    let schema_2 = r#"
    datasource ps {
        provider = "sqlserver"
        planetScaleMode = true
        url = "mysql://root:prisma@localhost:3306/mydb"
    }

    generator client {
        provider = "prisma-client-js"
    }
    "#;

    for schema in &[schema_1, schema_2] {
        let err = parse_error(schema);

        assert!(
            err.errors
                .first()
                .unwrap()
                .to_string()
                .starts_with("Error validating datasource `ps`: \nThe `planetScaleMode` option can only be set if the preview feature is enabled"),
            "{}",
            err.errors.first().unwrap()
        );
    }
}

#[test]
fn planet_scale_mode_with_preview_feature_works() {
    let schema = r#"
    datasource ps {
        provider = "sqlserver"
        planetScaleMode = true
        url = "mysql://root:prisma@localhost:3306/mydb"
    }

    generator client {
        provider = "prisma-client-js"
        previewFeatures = ["planetScaleMode"]
    }
    "#;

    let config = parse_configuration(schema);

    assert!(config.datasources[0].planet_scale_mode);
    assert!(config.planet_scale_mode());
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

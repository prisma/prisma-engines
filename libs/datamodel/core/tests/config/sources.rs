use crate::common::*;
use datamodel::StringFromEnvVar;
use datamodel_connector::ReferentialIntegrity;
use pretty_assertions::assert_eq;

#[test]
fn must_error_if_multiple_datasources_are_defined() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "postgresql"
          url = "postgresql://localhost"
        }

        datasource db2 {
          provider = "mysql"
          url = "mysql://localhost"
        }
    "#};

    let error = super::parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `db1`: You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mdatasource db1 {[0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = "postgresql://localhost"
        [1;94m 4 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating datasource `db2`: You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.[0m
          [1;94m-->[0m  [4mschema.prisma:6[0m
        [1;94m   | [0m
        [1;94m 5 | [0m
        [1;94m 6 | [0m[1;91mdatasource db2 {[0m
        [1;94m 7 | [0m  provider = "mysql"
        [1;94m 8 | [0m  url = "mysql://localhost"
        [1;94m 9 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_forbid_env_functions_in_provider_field() {
    let dml = indoc! {r#"
        datasource ds {
          provider = env("DB_PROVIDER")
          url = env("DB_URL")
        }
    "#};

    let error = super::parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mA datasource must not use the env() function in the provider argument.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mdatasource ds {[0m
        [1;94m 2 | [0m  provider = env("DB_PROVIDER")
        [1;94m 3 | [0m  url = env("DB_URL")
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_forbid_env_functions_in_provider_field_even_if_missing() {
    let dml = indoc! {r#"
        datasource ds {
          provider = env("DB_PROVIDER")
          url = env("DB_URL")
        }
    "#};

    let error = super::parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mA datasource must not use the env() function in the provider argument.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mdatasource ds {[0m
        [1;94m 2 | [0m  provider = env("DB_PROVIDER")
        [1;94m 3 | [0m  url = env("DB_URL")
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_for_empty_urls() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "sqlite"
          url = ""
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: You must provide a nonempty URL[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  url = [1;91m""[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_for_empty_provider_arrays() {
    let dml = indoc! {r#"
        datasource myds {
          provider = []
          url = "postgres://"
        }
    "#};

    let error = super::parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: The provider argument in a datasource must be a string literal[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mdatasource myds {
        [1;94m 2 | [0m  provider = [1;91m[][0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_for_empty_urls_derived_load_env_vars() {
    std::env::set_var("DB_URL_EMPTY_0001", "  ");

    let dml = indoc! {r#"
        datasource myds {
          provider = "sqlite"
          url = env("DB_URL_EMPTY_0001")
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: You must provide a nonempty URL. The environment variable `DB_URL_EMPTY_0001` resolved to an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  url = [1;91menv("DB_URL_EMPTY_0001")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_mysql() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "mysql"
          url = "postgresql://"
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the URL must start with the protocol `mysql://`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "mysql"
        [1;94m 3 | [0m  url = [1;91m"postgresql://"[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_mysql_shadow_database_url() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "mysql"
          url = "mysql://"
          shadowDatabaseUrl = "postgresql://"
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_shadow_database_url()
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the shadow database URL must start with the protocol `mysql://`.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  url = "mysql://"
        [1;94m 4 | [0m  shadowDatabaseUrl = [1;91m"postgresql://"[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_not_error_for_empty_shadow_database_urls_derived_load_env_vars() {
    std::env::set_var("EMPTY_SHADOW_DB URL_0129", "  ");

    let schema = indoc! {r#"
        datasource myds {
          provider = "postgres"
          url = "postgres://"
          shadowDatabaseUrl = env("EMPTY_SHADOW_DB URL_0129")
        }
    "#};

    let config = datamodel::parse_configuration(schema).unwrap();
    let shadow_database_url = config.subject.datasources[0].load_shadow_database_url().unwrap();

    assert!(shadow_database_url.is_none());
}

#[test]
fn must_not_error_for_shadow_database_urls_derived_from_missing_env_vars() {
    let schema = indoc! {r#"
        datasource myds {
          provider = "postgres"
          url = "postgres://"
          shadowDatabaseUrl = env("SHADOW_DATABASE_URL_NOT_SET_21357")
        }
    "#};

    let config = datamodel::parse_configuration(schema).unwrap();
    let shadow_database_url = config.subject.datasources[0].load_shadow_database_url().unwrap();

    assert!(shadow_database_url.is_none());
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_postgresql() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "postgresql"
          url = "mysql://"
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the URL must start with the protocol `postgresql://` or `postgres://`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = [1;91m"mysql://"[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_postgresql_shadow_database_url() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "postgresql"
          url = "postgresql://"
          shadowDatabaseUrl = "mysql://"
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_shadow_database_url()
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the shadow database URL must start with the protocol `postgresql://` or `postgres://`.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  url = "postgresql://"
        [1;94m 4 | [0m  shadowDatabaseUrl = [1;91m"mysql://"[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_wrong_protocol_is_used_for_sqlite() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "sqlite"
          url = "mysql://"
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the URL must start with the protocol `file:`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  url = [1;91m"mysql://"[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn new_lines_in_source_must_work() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = "postgresql://localhost"

        }
    "#};

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::render_sources_to_json(&config.datasources);

    let expected = expect![[r#"
        [
          {
            "name": "ds",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "url": {
              "fromEnvVar": null,
              "value": "postgresql://localhost"
            }
          }
        ]"#]];

    expected.assert_eq(&rendered);
}

#[test]
fn must_error_if_env_var_is_missing() {
    let dml = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env("MISSING_DATABASE_URL_0001")
        }
    "#};

    let config = super::parse_config(dml).unwrap();

    let error = config.subject.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mEnvironment variable not found: MISSING_DATABASE_URL_0001.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgresql"
        [1;94m 3 | [0m  url = [1;91menv("MISSING_DATABASE_URL_0001")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_succeed_if_env_var_is_missing_but_override_was_provided() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env("MISSING_DATABASE_URL_0002")
        }
    "#};

    let url = "postgres://localhost";
    let overrides = vec![("ds".to_string(), url.to_string())];
    let mut config = parse_configuration(schema);
    config
        .resolve_datasource_urls_from_env(&overrides, load_env_var)
        .unwrap();
    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        value: Some(url.to_string()),
        from_env_var: None,
    });
}

#[test]
fn must_succeed_if_env_var_exists_and_override_was_provided() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env("DATABASE_URL")
        }
    "#};

    std::env::set_var("DATABASE_URL", "postgres://hostfoo");

    let url = "postgres://hostbar";
    let overrides = vec![("ds".to_string(), url.to_string())];
    let mut config = parse_configuration(schema);

    config
        .resolve_datasource_urls_from_env(&overrides, load_env_var)
        .unwrap();

    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    assert_eq!(data_source.url.value.as_deref(), Some(url));

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DATABASE_URL");
}

#[test]
fn must_succeed_with_overrides() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = "postgres://hostfoo"
        }
    "#};

    let url = "postgres://hostbar";
    let overrides = &[("ds".to_string(), url.to_string())];
    let mut config = parse_configuration(schema);

    config
        .resolve_datasource_urls_from_env(overrides, load_env_var)
        .unwrap();

    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    assert_eq!(data_source.url.value.as_deref(), Some(url));
}

#[test]
fn fail_to_load_sources_for_invalid_source() {
    let dml = indoc! {r#"
        datasource pg1 {
          provider = "AStrangeHalfMongoDatabase"
          url = "https://localhost/postgres1"
        }
    "#};

    let error = super::parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mDatasource provider not known: "AStrangeHalfMongoDatabase".[0m
          [1;94m-->[0m  [4mschema.prisma:2[0m
        [1;94m   | [0m
        [1;94m 1 | [0mdatasource pg1 {
        [1;94m 2 | [0m  provider = [1;91m"AStrangeHalfMongoDatabase"[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fail_when_preview_features_are_declared() {
    let dml = indoc! {r#"
        datasource db {
          provider = "mysql"
          url = "mysql://"
          previewFeatures = ["foo"]
        }
    "#};

    let error = super::parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mPreview features are only supported in the generator block. Please move this field to the generator block.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  url = "mysql://"
        [1;94m 4 | [0m  [1;91mpreviewFeatures = ["foo"][0m
        [1;94m 5 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fail_when_no_source_is_declared() {
    let invalid_datamodel: &str = r#"        "#;

    let error = datamodel::parse_configuration(invalid_datamodel)
        .and_then(|res| res.subject.validate_that_one_datasource_is_provided())
        .map_err(|e| e.to_pretty_string("schema.prisma", invalid_datamodel))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating: You defined no datasource. You must define exactly one datasource.[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91m[0m        
        [1;94m   | [0m[1;91m^ Unexpected token.[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn referential_integrity_without_preview_feature_errors() {
    let schema = indoc! {r#"
        datasource ps {
          provider = "sqlserver"
          referentialIntegrity = "prisma"
          url = "mysql://root:prisma@localhost:3306/mydb"
        }

        generator client {
          provider = "prisma-client-js"
        }
    "#};

    let error = super::parse_config(schema).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `ps`: 
        The `referentialIntegrity` option can only be set if the preview feature is enabled in a generator block.

        Example:

        generator client {
            provider = "prisma-client-js"
            previewFeatures = ["referentialIntegrity"]
        }
        [0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlserver"
        [1;94m 3 | [0m  [1;91mreferentialIntegrity = "prisma"[0m
        [1;94m 4 | [0m  url = "mysql://root:prisma@localhost:3306/mydb"
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn referential_integrity_with_preview_feature_works() {
    let schema = indoc! {r#"
        datasource ps {
          provider = "sqlserver"
          referentialIntegrity = "prisma"
          url = "mysql://root:prisma@localhost:3306/mydb"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }
    "#};

    let config = parse_configuration(schema);

    assert_eq!(config.referential_integrity(), Some(ReferentialIntegrity::Prisma));
}

#[test]
fn referential_integrity_default() {
    let schema = indoc! {r#"
        datasource ps {
          provider = "sqlserver"
          url = "mysql://root:prisma@localhost:3306/mydb"
        }

        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["referentialIntegrity"]
        }
    "#};

    let config = parse_configuration(schema);

    assert_eq!(config.referential_integrity(), Some(ReferentialIntegrity::ForeignKeys));
}

#[test]
fn cockroach_provider_is_behind_preview_feature() {
    let dm = r#"
        datasource ps {
          provider = "cockroachdb"
          url = env("DATABASE_URL")
        }
    "#;

    let error = super::parse_config(dm).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mDatasource provider not known: "cockroachdb".[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m        datasource ps {
        [1;94m 3 | [0m          provider = [1;91m"cockroachdb"[0m
        [1;94m   | [0m
    "#]];
    expectation.assert_eq(&error);
}

#[test]
fn mongo_provider_is_behind_preview_feature() {
    let dm = r#"
        datasource ps {
          provider = "mongodb"
          url = env("DATABASE_URL")
        }
    "#;

    let error = super::parse_config(dm).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mDatasource provider not known: "mongodb".[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m        datasource ps {
        [1;94m 3 | [0m          provider = [1;91m"mongodb"[0m
        [1;94m   | [0m
    "#]];
    expectation.assert_eq(&error);
}

fn load_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

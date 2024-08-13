use crate::common::*;
use psl::{datamodel_connector::RelationMode, StringFromEnvVar};

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

    let error = parse_config(dml).map(drop).unwrap_err();

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

    let error = parse_config(dml).map(drop).unwrap_err();

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

    let error = parse_config(dml).map(drop).unwrap_err();

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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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

    let error = parse_config(dml).map(drop).unwrap_err();

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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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
    std::env::remove_var("DB_URL_EMPTY_0001");

    expectation.assert_eq(&error)
}

#[test]
fn must_error_if_prisma_protocol_is_used_for_mysql() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "mysql"
          url = "prisma://"
        }
    "#};

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the URL must start with the protocol `mysql://`.

        To use a URL with protocol `prisma://`, you need to either enable Accelerate or the Data Proxy.
        Enable Accelerate via `prisma generate --accelerate` or the Data Proxy via `prisma generate --data-proxy.`

        More information about Data Proxy: https://pris.ly/d/data-proxy
        [0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "mysql"
        [1;94m 3 | [0m  url = [1;91m"prisma://"[0m
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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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

    let config = parse_configuration(schema);
    let shadow_database_url = config.datasources[0].load_shadow_database_url().unwrap();

    std::env::remove_var("EMPTY_SHADOW_DB URL_0129");
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

    let config = parse_configuration(schema);
    let shadow_database_url = config.datasources[0].load_shadow_database_url().unwrap();

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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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

    let rendered = render_datasources(schema);

    let expected = expect![[r#"
        [
          {
            "name": "ds",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "url": {
              "fromEnvVar": null,
              "value": "postgresql://localhost"
            },
            "schemas": [],
            "sourceFilePath": "schema.prisma"
          }
        ]"#]];

    expected.assert_eq(&rendered);
}

#[test]
fn multischema_must_work() {
    let schema = indoc! {r#"
      generator client {
        provider        = "prisma-client-js"
        previewFeatures = ["multiSchema"]
      }

      datasource ds {
        provider = "postgresql"
        url = "postgresql://localhost"
        schemas = ["transactional", "public"]
      }
    "#};

    let rendered = render_datasources(schema);

    // schemas are sorted in ascending order
    let expected = expect![[r#"
        [
          {
            "name": "ds",
            "provider": "postgresql",
            "activeProvider": "postgresql",
            "url": {
              "fromEnvVar": null,
              "value": "postgresql://localhost"
            },
            "schemas": [
              "public",
              "transactional"
            ],
            "sourceFilePath": "schema.prisma"
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

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
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
        .resolve_datasource_urls_query_engine(&overrides, load_env_var, false)
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
        .resolve_datasource_urls_query_engine(&overrides, load_env_var, false)
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
        .resolve_datasource_urls_query_engine(overrides, load_env_var, false)
        .unwrap();

    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    assert_eq!(data_source.url.value.as_deref(), Some(url));
}

#[test]
fn must_succeed_when_ignoring_env_errors_and_retain_env_var_name() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env("MISSING_DATABASE_URL_0003")
        }
    "#};

    let mut config = parse_configuration(schema);

    config
        .resolve_datasource_urls_query_engine(&[], load_env_var, true)
        .unwrap();

    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        value: None,
        from_env_var: Some("MISSING_DATABASE_URL_0003".to_string()),
    });
}

#[test]
fn must_process_overrides_when_ignoring_env_errors() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"
          url = env("MISSING_DATABASE_URL_0004")
        }
    "#};

    let url = "postgres://localhost".to_string();
    let overrides = vec![("ds".to_string(), url.clone())];
    let mut config = parse_configuration(schema);

    config
        .resolve_datasource_urls_query_engine(&overrides, load_env_var, true)
        .unwrap();

    let data_source = config.datasources.first().unwrap();

    data_source.assert_name("ds");
    data_source.assert_url(StringFromEnvVar {
        value: Some(url),
        from_env_var: None,
    });
}

#[test]
fn fail_to_load_sources_for_invalid_source() {
    let dml = indoc! {r#"
        datasource pg1 {
          provider = "AStrangeHalfMongoDatabase"
          url = "https://localhost/postgres1"
        }
    "#};

    let error = parse_config(dml).map(drop).unwrap_err();

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

    let error = parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mPreview features are only supported in the generator block. Please move this field to the generator block.[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  url = "mysql://"
        [1;94m 4 | [0m  [1;91mpreviewFeatures = ["foo"][0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn fail_when_no_source_is_declared() {
    let invalid_datamodel: &str = r#"        "#;

    let error = psl::parse_configuration(invalid_datamodel)
        .and_then(|res| res.validate_that_one_datasource_is_provided())
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
fn referential_integrity_works() {
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

    let config = parse_configuration(schema);

    assert_eq!(config.relation_mode(), Some(RelationMode::Prisma));
}

#[test]
fn relation_mode_works() {
    let schema = indoc! {r#"
        datasource ps {
          provider = "sqlserver"
          relationMode = "prisma"
          url = "mysql://root:prisma@localhost:3306/mydb"
        }

        generator client {
          provider = "prisma-client-js"
        }
    "#};

    let config = parse_configuration(schema);

    assert_eq!(config.relation_mode(), Some(RelationMode::Prisma));
}

#[test]
fn relation_mode_default() {
    let schema = indoc! {r#"
        datasource ps {
          provider = "sqlserver"
          url = "mysql://root:prisma@localhost:3306/mydb"
        }

        generator client {
          provider = "prisma-client-js"
        }
    "#};

    let config = parse_configuration(schema);

    assert_eq!(config.relation_mode(), Some(RelationMode::ForeignKeys));
}

fn load_env_var(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

#[test]
fn must_error_for_empty_direct_urls() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "sqlite"
          directUrl = ""
          url = "file://hostfoo"
        }
    "#};

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_direct_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: You must provide a nonempty direct URL[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  directUrl = [1;91m""[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_for_empty_env_direct_urls() {
    std::env::set_var("DB_DIRECT_URL_EMPTY_0001", "  ");
    let dml = indoc! {r#"
        datasource myds {
          provider = "sqlite"
          directUrl = env("DB_DIRECT_URL_EMPTY_0001")
          url = "file://hostfoo"
        }
    "#};

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_direct_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mError validating datasource `myds`: You must provide a nonempty direct URL. The environment variable `DB_DIRECT_URL_EMPTY_0001` resolved to an empty string.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  directUrl = [1;91menv("DB_DIRECT_URL_EMPTY_0001")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_for_missing_env_direct_urls() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "sqlite"
          directUrl = env("MISSING_DIRECT_ENV_VAR_0001")
          url = "file://hostfoo"
        }
    "#};

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_direct_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mEnvironment variable not found: MISSING_DIRECT_ENV_VAR_0001.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "sqlite"
        [1;94m 3 | [0m  directUrl = [1;91menv("MISSING_DIRECT_ENV_VAR_0001")[0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn directurl_should_work_with_proxy_url() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "postgres"
          directUrl = env("DATABASE_URL_0001")
          url = "prisma://localhost:1234"
        }
    "#};

    std::env::set_var("DATABASE_URL_0001", "postgres://hostfoo");

    let config = parse_config(dml).unwrap();

    let result = config.datasources[0]
        .load_direct_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap();

    let expectation = expect!("postgres://hostfoo");

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DATABASE_URL_0001");

    expectation.assert_eq(&result)
}

#[test]
fn load_url_should_not_work_with_proxy_url() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "postgres"
          directUrl = env("DIRECT_URL_0002")
          url = env("DATABASE_URL_0002")
        }
    "#};

    std::env::set_var("DATABASE_URL_0002", "prisma://hostbar");
    std::env::set_var("DIRECT_URL_0002", "postgres://hostfoo");

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect!([r#"
        [1;91merror[0m: [1mError validating datasource `myds`: the URL must start with the protocol `postgresql://` or `postgres://`.

        To use a URL with protocol `prisma://`, you need to either enable Accelerate or the Data Proxy.
        Enable Accelerate via `prisma generate --accelerate` or the Data Proxy via `prisma generate --data-proxy.`

        More information about Data Proxy: https://pris.ly/d/data-proxy
        [0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m  directUrl = env("DIRECT_URL_0002")
        [1;94m 4 | [0m  url = [1;91menv("DATABASE_URL_0002")[0m
        [1;94m   | [0m
    "#]);

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DATABASE_URL_0002");
    std::env::remove_var("DIRECT_URL_0002");

    expectation.assert_eq(&error)
}

#[test]
fn load_url_no_validation_should_work_with_proxy_url() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "postgres"
          directUrl = env("DIRECT_URL_0003")
          url = env("DATABASE_URL_0003")
        }
    "#};

    std::env::set_var("DATABASE_URL_0003", "prisma://hostbar");
    std::env::set_var("DIRECT_URL_0003", "postgres://hostfoo");

    let config = parse_config(dml).unwrap();

    let result = config.datasources[0]
        .load_url_no_validation(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap();

    let expectation = expect!("prisma://hostbar");

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DATABASE_URL_0003");
    std::env::remove_var("DIRECT_URL_0003");

    expectation.assert_eq(&result)
}

#[test]
fn directurl_should_not_use_prisma_scheme_when_using_env_vars() {
    std::env::set_var("DATABASE_URL_0004", "prisma://hostbar");
    std::env::set_var("DIRECT_URL_0004", "prisma://hostfoo");

    let dml = indoc! {r#"
        datasource myds {
          provider = "postgres"
          directUrl = env("DIRECT_URL_0004")
          url = env("DATABASE_URL_0004")
        }
    "#};

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_direct_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect!([r#"
        [1;91merror[0m: [1mError validating datasource `myds`: You must provide a direct URL that points directly to the database. Using `prisma` in URL scheme is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgres"
        [1;94m 3 | [0m  directUrl = [1;91menv("DIRECT_URL_0004")[0m
        [1;94m   | [0m
    "#]);

    expectation.assert_eq(&error);

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DIRECT_URL_0004");
    std::env::remove_var("DATABASE_URL_0004");
}

#[test]
fn directurl_should_not_use_prisma_scheme() {
    let dml = indoc! {r#"
        datasource myds {
          provider = "postgres"
          directUrl = "prisma://kekw.lol"
          url = env("DATABASE_URL_0005")
        }
    "#};

    std::env::set_var("DATABASE_URL_0005", "prisma://hostbar");

    let config = parse_config(dml).unwrap();

    let error = config.datasources[0]
        .load_direct_url(load_env_var)
        .map_err(|e| e.to_pretty_string("schema.prisma", dml))
        .unwrap_err();

    let expectation = expect!([r#"
        [1;91merror[0m: [1mError validating datasource `myds`: You must provide a direct URL that points directly to the database. Using `prisma` in URL scheme is not allowed.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "postgres"
        [1;94m 3 | [0m  directUrl = [1;91m"prisma://kekw.lol"[0m
        [1;94m   | [0m
    "#]);

    expectation.assert_eq(&error);

    // make sure other tests that run afterwards are not run in a modified environment
    std::env::remove_var("DIRECT_URL_0005");
}

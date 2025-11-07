use crate::common::*;
use psl::datamodel_connector::RelationMode;

#[test]
fn must_error_if_multiple_datasources_are_defined() {
    let dml = indoc! {r#"
        datasource db1 {
          provider = "postgresql"
        }

        datasource db2 {
          provider = "mysql"
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
        [1;94m 3 | [0m}
        [1;94m   | [0m
        [1;91merror[0m: [1mError validating datasource `db2`: You defined more than one datasource. This is not allowed yet because support for multiple databases has not been implemented yet.[0m
          [1;94m-->[0m  [4mschema.prisma:5[0m
        [1;94m   | [0m
        [1;94m 4 | [0m
        [1;94m 5 | [0m[1;91mdatasource db2 {[0m
        [1;94m 6 | [0m  provider = "mysql"
        [1;94m 7 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_forbid_env_functions_in_provider_field() {
    let dml = indoc! {r#"
        datasource ds {
          provider = env("DB_PROVIDER")
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
        [1;94m 3 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn must_error_for_empty_provider_arrays() {
    let dml = indoc! {r#"
        datasource myds {
          provider = []
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
fn new_lines_in_source_must_work() {
    let schema = indoc! {r#"
        datasource ds {
          provider = "postgresql"

        }
    "#};

    let rendered = render_datasources(schema);

    let expected = expect![[r#"
        [
          {
            "name": "ds",
            "provider": "postgresql",
            "activeProvider": "postgresql",
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
        provider        = "prisma-client"
        previewFeatures = []
      }

      datasource ds {
        provider = "postgresql"
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
fn fail_to_load_sources_for_invalid_source() {
    let dml = indoc! {r#"
        datasource pg1 {
          provider = "AStrangeHalfMongoDatabase"
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
          previewFeatures = ["foo"]
        }
    "#};

    let error = parse_config(dml).map(drop).unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mPreview features are only supported in the generator block. Please move this field to the generator block.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "mysql"
        [1;94m 3 | [0m  [1;91mpreviewFeatures = ["foo"][0m
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
        }

        generator client {
          provider = "prisma-client"
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
        }

        generator client {
          provider = "prisma-client"
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
        }

        generator client {
          provider = "prisma-client"
        }
    "#};

    let config = parse_configuration(schema);

    assert_eq!(config.relation_mode(), Some(RelationMode::ForeignKeys));
}

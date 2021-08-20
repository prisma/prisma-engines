use crate::common::*;
use expect_test::expect;

#[test]
fn serialize_generators_to_cmf() {
    let schema: &str = indoc! {r#"
        generator js1 {
          provider = "javascript"
          output = "../../js"
        }

        generator go {
          provider = "go"
          binaryTargets = ["a", "b"]
        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "js1",
            "provider": {
              "fromEnvVar": null,
              "value": "javascript"
            },
            "output": {
              "fromEnvVar": null,
              "value": "../../js"
            },
            "config": {},
            "binaryTargets": [],
            "previewFeatures": []
          },
          {
            "name": "go",
            "provider": {
              "fromEnvVar": null,
              "value": "go"
            },
            "output": null,
            "config": {},
            "binaryTargets": [
              {
                "fromEnvVar": null,
                "value": "a"
              },
              {
                "fromEnvVar": null,
                "value": "b"
              }
            ],
            "previewFeatures": []
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}

#[test]
fn preview_features_setting_must_work() {
    // make sure both single value and array syntax work
    let schema = indoc! {r#"
        generator js {
          provider = "javascript"
          previewFeatures = "connectOrCreate"
        }

        generator go {
          provider = "go"
          previewFeatures = ["connectOrCreate", "transactionApi"]
        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "js",
            "provider": {
              "fromEnvVar": null,
              "value": "javascript"
            },
            "output": null,
            "config": {},
            "binaryTargets": [],
            "previewFeatures": [
              "connectOrCreate"
            ]
          },
          {
            "name": "go",
            "provider": {
              "fromEnvVar": null,
              "value": "go"
            },
            "output": null,
            "config": {},
            "binaryTargets": [],
            "previewFeatures": [
              "connectOrCreate",
              "transactionApi"
            ]
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}

#[test]
fn hidden_preview_features_setting_must_work() {
    let schema = indoc! {r#"
        generator go {
          provider = "go"
          previewFeatures = ["mongoDb"]
        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "go",
            "provider": {
              "fromEnvVar": null,
              "value": "go"
            },
            "output": null,
            "config": {},
            "binaryTargets": [],
            "previewFeatures": [
              "mongoDb"
            ]
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}
#[test]
fn back_slashes_in_providers_must_work() {
    let schema = indoc! {r#"
        generator mygen {
          provider = "../folder\ with\\ space/my\ generator.js"
        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "mygen",
            "provider": {
              "fromEnvVar": null,
              "value": "../folder with\\ space/my generator.js"
            },
            "output": null,
            "config": {},
            "binaryTargets": [],
            "previewFeatures": []
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}

#[test]
fn new_lines_in_generator_must_work() {
    let schema = indoc! {r#"
        generator go {
          provider = "go"
          binaryTargets = ["b", "c"]

        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "go",
            "provider": {
              "fromEnvVar": null,
              "value": "go"
            },
            "output": null,
            "config": {},
            "binaryTargets": [
              {
                "fromEnvVar": null,
                "value": "b"
              },
              {
                "fromEnvVar": null,
                "value": "c"
              }
            ],
            "previewFeatures": []
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}

#[test]
fn fail_to_load_generator_with_options_missing() {
    let schema = indoc! {r#"
        generator js1 {
          no_provider = "javascript"
          output = "../../js"
        }
    "#};

    let error = datamodel::parse_configuration(schema)
        .map(drop)
        .map_err(|diag| diag.to_pretty_string("schema.prisma", schema))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mArgument "provider" is missing in generator block "js1".[0m
          [1;94m-->[0m  [4mschema.prisma:1[0m
        [1;94m   | [0m
        [1;94m   | [0m
        [1;94m 1 | [0m[1;91mgenerator js1 {[0m
        [1;94m 2 | [0m  no_provider = "javascript"
        [1;94m 3 | [0m  output = "../../js"
        [1;94m 4 | [0m}
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn nice_error_for_unknown_generator_preview_feature() {
    let schema = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
          previewFeatures = ["foo"]
        }
    "#};

    let error = datamodel::parse_configuration(schema)
        .map(drop)
        .map_err(|diag| diag.to_pretty_string("schema.prisma", schema))
        .unwrap_err();

    let expectation = expect![[r#"
        [1;91merror[0m: [1mThe preview feature "foo" is not known. Expected one of: microsoftSqlServer, orderByRelation, nApi, selectRelationCount, orderByAggregateGroup, filterJson, planetScaleMode, referentialActions, mongoDb, interactiveTransactions, namedConstraints, fullTextSearch[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "prisma-client-js"
        [1;94m 3 | [0m  previewFeatures = [1;91m["foo"][0m
        [1;94m   | [0m
    "#]];

    expectation.assert_eq(&error)
}

#[test]
fn binary_targets_from_env_var_should_work() {
    let schema = indoc! {r#"
        datasource db {
          provider = "mysql"
          url      = env("DATABASE_URL")
        }

        generator client {
          provider      = "prisma-client-js"
          binaryTargets = env("BINARY_TARGETS")
        }

        model User {
          id Int @id
        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "client",
            "provider": {
              "fromEnvVar": null,
              "value": "prisma-client-js"
            },
            "output": null,
            "config": {},
            "binaryTargets": [
              {
                "fromEnvVar": "BINARY_TARGETS",
                "value": null
              }
            ],
            "previewFeatures": []
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}

#[test]
fn retain_env_var_definitions_in_generator_block() {
    let schema = indoc! {r#"
        generator js1 {
          provider = env("PROVIDER")
          output = env("OUTPUT")
        }
    "#};

    let expected = expect![[r#"
        [
          {
            "name": "js1",
            "provider": {
              "fromEnvVar": "PROVIDER",
              "value": null
            },
            "output": {
              "fromEnvVar": "OUTPUT",
              "value": null
            },
            "config": {},
            "binaryTargets": [],
            "previewFeatures": []
          }
        ]"#]];

    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    expected.assert_eq(&rendered);
}

#[test]
fn env_in_preview_features_must_be_rejected() {
    let schema_1 = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
          previewFeatures = [env("MY_PREVIEW_FEATURE")]
        }
    "#};

    let schema_2 = indoc! {r#"
        generator client {
          provider = "prisma-client-js"
          previewFeatures = env("MY_PREVIEW_FEATURE")
        }
    "#};

    let expect_1 = expect![[r#"
        [1;91merror[0m: [1mExpected a String value, but received functional value `env("MY_PREVIEW_FEATURE")`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "prisma-client-js"
        [1;94m 3 | [0m  previewFeatures = [[1;91menv("MY_PREVIEW_FEATURE")[0m]
        [1;94m   | [0m
    "#]];

    let expect_2 = expect![[r#"
        [1;91merror[0m: [1mExpected a String value, but received functional value `env("MY_PREVIEW_FEATURE")`.[0m
          [1;94m-->[0m  [4mschema.prisma:3[0m
        [1;94m   | [0m
        [1;94m 2 | [0m  provider = "prisma-client-js"
        [1;94m 3 | [0m  previewFeatures = [1;91menv("MY_PREVIEW_FEATURE")[0m
        [1;94m   | [0m
    "#]];

    expect_1.assert_eq(&datamodel::parse_schema(schema_1).map(drop).unwrap_err());
    expect_2.assert_eq(&datamodel::parse_schema(schema_2).map(drop).unwrap_err());
}

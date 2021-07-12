use crate::common::{parse_configuration, ErrorAsserts};
use datamodel::common::preview_features::GENERATOR;
use datamodel::diagnostics::DatamodelError;
use expect_test::expect;
use itertools::Itertools;
use pretty_assertions::assert_eq;

#[test]
fn serialize_generators_to_cmf() {
    let schema: &str = r#"
generator js1 {
    provider = "javascript"
    output = "../../js"
}

generator go {
    provider = "go"
    binaryTargets = ["a", "b"]
}"#;

    let expected = r#"[
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
    "binaryTargets": [],
    "previewFeatures": [],
    "config": {}
  },
  {
    "name": "go",
    "provider": {
        "fromEnvVar": null,
        "value": "go"
    },
    "output": null,
    "binaryTargets": [{ "value": "a", "fromEnvVar": null }, { "value": "b", "fromEnvVar": null }],
    "previewFeatures": [],
    "config": {}
  }
]"#;

    assert_mcf(schema, expected);
}

#[test]
fn preview_features_setting_must_work() {
    // make sure both single value and array syntax work
    let schema = r#"
        generator js {
            provider = "javascript"
            previewFeatures = "connectOrCreate"
        }

        generator go {
            provider = "go"
            previewFeatures = ["connectOrCreate", "transactionApi"]
        }
    "#;

    let expected = r#"[
  {
    "name": "js",
    "provider": {
        "fromEnvVar": null,
        "value": "javascript"
    },
    "output":null,
    "binaryTargets": [],
    "previewFeatures": ["connectOrCreate"],
    "config": {}
  },
  {
    "name": "go",
    "provider": {
        "fromEnvVar": null,
        "value": "go"
    },
    "output":null,
    "binaryTargets": [],
    "previewFeatures": ["connectOrCreate", "transactionApi"],
    "config": {}
  }
]"#;

    assert_mcf(schema, expected);
}

#[test]
fn hidden_preview_features_setting_must_work() {
    let schema = r#"
        generator go {
            provider = "go"
            previewFeatures = ["mongoDb"]
        }
    "#;

    let expected = r#"[
  {
    "name": "go",
    "provider": {
        "fromEnvVar": null,
        "value": "go"
    },
    "output":null,
    "config": {},
    "binaryTargets": [],
    "previewFeatures": ["mongoDb"]
  }
]"#;

    assert_mcf(schema, expected);
}
#[test]
fn back_slashes_in_providers_must_work() {
    let schema = r#"
        generator mygen {
          provider = "../folder\ with\\ space/my\ generator.js"
        }
    "#;

    let expected = r#"[
        {
          "name": "mygen",
          "provider":{
            "fromEnvVar": null,
            "value": "../folder with\\ space/my generator.js"
          },
          "output": null,
          "binaryTargets": [],
          "previewFeatures": [],
          "config": {}
        }
    ]"#;

    assert_mcf(schema, expected);
}

#[test]
fn new_lines_in_generator_must_work() {
    let schema = r#"
        generator go {
          provider = "go"
          binaryTargets = ["b", "c"]

        }
    "#;

    let expected = r#"[
        {
          "name": "go",
          "provider": {
            "fromEnvVar": null,
            "value": "go"
          },
          "output": null,
          "binaryTargets": [{ "value": "b", "fromEnvVar": null }, { "value": "c", "fromEnvVar": null }],
          "previewFeatures": [],
          "config": {}
        }
    ]"#;

    assert_mcf(schema, expected);
}

#[test]
fn fail_to_load_generator_with_options_missing() {
    let schema = r#"
generator js1 {
    no_provider = "javascript"
    output = "../../js"
}
    "#;
    let res = datamodel::parse_configuration(schema);

    if let Err(diagnostics) = res {
        diagnostics.assert_is(DatamodelError::GeneratorArgumentNotFound {
            argument_name: String::from("provider"),
            generator_name: String::from("js1"),
            span: datamodel::ast::Span::new(1, 73),
        });
    } else {
        panic!("Expected error.")
    }
}

#[test]
fn nice_error_for_unknown_generator_preview_feature() {
    let schema = r#"
    generator client {
      provider = "prisma-client-js"
      previewFeatures = ["foo"]
    }
    "#;

    let res = datamodel::parse_configuration(schema);

    if let Err(diagnostics) = res {
        diagnostics.assert_is(DatamodelError::new_preview_feature_not_known_error(
            "foo",
            GENERATOR.active_features().iter().map(ToString::to_string).join(", "),
            datamodel::ast::Span::new(84, 91),
        ));
    } else {
        panic!("Expected error.")
    }
}

#[test]
fn binary_targets_from_env_var_should_work() {
    let schema = r#"
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
    "#;

    let expected_dmmf = r#"[
        {
          "name": "client",
          "provider": {
            "fromEnvVar": null,
            "value": "prisma-client-js"
          },
          "output": null,
          "binaryTargets": [
            {
                "fromEnvVar": "BINARY_TARGETS",
                "value": null
            }
          ],
          "previewFeatures": [],
          "config": {}
        }
      ]"#;

    assert_mcf(schema, expected_dmmf);
}

#[test]
fn retain_env_var_definitions_in_generator_block() {
    let schema1 = r#"
    generator js1 {
        provider = env("PROVIDER")
        output = env("OUTPUT")
    }
    "#;

    let expected_dmmf_1 = r#"[
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
    "binaryTargets": [],
    "previewFeatures": [],
    "config": {}
  }
]"#;

    assert_mcf(schema1, expected_dmmf_1);
}

#[test]
fn env_in_preview_features_must_be_rejected() {
    let schema_1 = r#"
        generator client {
            provider = "prisma-client-js"
            previewFeatures = [env("MY_PREVIEW_FEATURE")]
        }
    "#;

    let schema_2 = r#"
        generator client {
            provider = "prisma-client-js"
            previewFeatures = env("MY_PREVIEW_FEATURE")
        }
    "#;

    let expect_1 = expect![[r#"
        [1;91merror[0m: [1mExpected a String value, but received functional value "env".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            provider = "prisma-client-js"
        [1;94m 4 | [0m            previewFeatures = [[1;91menv("MY_PREVIEW_FEATURE")[0m]
        [1;94m   | [0m
    "#]];

    let expect_2 = expect![[r#"
        [1;91merror[0m: [1mExpected a String value, but received functional value "env".[0m
          [1;94m-->[0m  [4mschema.prisma:4[0m
        [1;94m   | [0m
        [1;94m 3 | [0m            provider = "prisma-client-js"
        [1;94m 4 | [0m            previewFeatures = [1;91menv("MY_PREVIEW_FEATURE")[0m
        [1;94m   | [0m
    "#]];

    expect_1.assert_eq(&datamodel::parse_schema(schema_1).map(drop).unwrap_err());
    expect_2.assert_eq(&datamodel::parse_schema(schema_2).map(drop).unwrap_err());
}

fn assert_mcf(schema: &str, expected_mcf: &str) {
    let config = parse_configuration(schema);
    let rendered = datamodel::json::mcf::generators_to_json(&config.generators);

    assert_eq_json(&rendered, expected_mcf);
}

fn assert_eq_json(a: &str, b: &str) {
    let json_a: serde_json::Value = serde_json::from_str(a).expect("The String a was not valid JSON.");
    let json_b: serde_json::Value = serde_json::from_str(b).expect("The String b was not valid JSON.");

    assert_eq!(json_a, json_b);
}

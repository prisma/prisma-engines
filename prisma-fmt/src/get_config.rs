use psl::{diagnostics::DatamodelError, error_tolerant_parse_configuration, parser_database::Files, Diagnostics};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::schema_file_input::SchemaFileInput;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetConfigParams {
    prisma_schema: SchemaFileInput,
    #[serde(default)]
    ignore_env_var_errors: bool,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    datasource_overrides: HashMap<String, String>,
}

#[derive(Serialize)]
struct GetConfigResult<'a> {
    config: serde_json::Value,
    errors: Vec<ValidationError<'a>>,
}

#[derive(Serialize)]
struct ValidationError<'a> {
    file_name: Option<&'a str>,
    message: String,
}

pub(crate) fn get_config(params: &str) -> String {
    let params: GetConfigParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize GetConfigParams: {serde_err}",);
        }
    };

    let schema: Vec<_> = params.prisma_schema.into();

    let (files, mut configuration, diagnostics) = error_tolerant_parse_configuration(&schema);

    let override_diagnostics = if params.ignore_env_var_errors {
        Diagnostics::default()
    } else {
        let overrides: Vec<(_, _)> = params.datasource_overrides.into_iter().collect();
        let override_result =
            configuration.resolve_datasource_urls_prisma_fmt(&overrides, |key| params.env.get(key).map(String::from));

        match override_result {
            Err(diagnostics) => diagnostics,
            _ => Diagnostics::default(),
        }
    };

    let config = psl::get_config(&configuration);
    let all_errors = diagnostics.errors().iter().chain(override_diagnostics.errors().iter());

    let result = GetConfigResult {
        config,
        errors: serialize_errors(all_errors, &files),
    };

    serde_json::to_string(&result).unwrap()
}

fn serialize_errors<'a>(
    errors: impl Iterator<Item = &'a DatamodelError>,
    files: &'a Files,
) -> Vec<ValidationError<'a>> {
    errors
        .map(move |error| {
            let file_id = error.span().file_id;
            let (file_name, source, _) = &files[file_id];
            let mut message_pretty: Vec<u8> = vec![];
            error.pretty_print(&mut message_pretty, file_name, source.as_str())?;

            Ok(ValidationError {
                file_name: Some(file_name),
                message: String::from_utf8_lossy(&message_pretty).into_owned(),
            })
        })
        .collect::<Result<Vec<_>, std::io::Error>>()
        .unwrap_or_else(|error| {
            vec![ValidationError {
                file_name: None,
                message: format!("Could not serialize validation errors: {error}"),
            }]
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use serde_json::json;

    #[test]
    fn invalid_schema() {
        let schema = r#"
            generator js {
            }

            datasøurce yolo {
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            \u001b[1;91mdatasøurce yolo {\u001b[0m\n\u001b[1;94m 6 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n"},{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            datasøurce yolo {\n\u001b[1;94m 6 | \u001b[0m            \u001b[1;91m}\u001b[0m\n\u001b[1;94m 7 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n"},{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mArgument \"provider\" is missing in generator block \"js\".\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:2\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 1 | \u001b[0m\n\u001b[1;94m 2 | \u001b[0m            \u001b[1;91mgenerator js {\u001b[0m\n\u001b[1;94m 3 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block() {
        let schema = r#"
            generator js {
                provider = "prisma-client-js"
                previewFeatures = ["prismaSchemaFolder"]
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client-js"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":["prismaSchemaFolder"]}],"datasources":[],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block_invalid_model() {
        let schema = r#"
            generator js {
                provider = "prisma-client-js"
                previewFeatures = ["prismaSchemaFolder"]
            }

            model M {
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client-js"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":["prismaSchemaFolder"]}],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:7\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 6 | \u001b[0m\n\u001b[1;94m 7 | \u001b[0m            \u001b[1;91mmodel M {\u001b[0m\n\u001b[1;94m 8 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block_invalid_model_field() {
        let schema = r#"
            generator js {
                provider = "prisma-client-js"
                previewFeatures = ["prismaSchemaFolder"]
            }

            model M {
                field
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client-js"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":["prismaSchemaFolder"]}],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating model \"M\": This field declaration is invalid. It is either missing a name or a type.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:8\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 7 | \u001b[0m            model M {\n\u001b[1;94m 8 | \u001b[0m                \u001b[1;91mfield\u001b[0m\n\u001b[1;94m 9 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block_invalid_datasource() {
        let schema = r#"
            generator js {
                provider = "prisma-client-js"
                previewFeatures = ["prismaSchemaFolder"]
            }

            datasource D {
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client-js"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":["prismaSchemaFolder"]}],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:7\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 6 | \u001b[0m\n\u001b[1;94m 7 | \u001b[0m            \u001b[1;91mdatasource D {\u001b[0m\n\u001b[1;94m 8 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn multifile() {
        let schemas = &[
            (
                "generator.prisma",
                r#"generator js {
                    provider = "prisma-client-js"
                    previewFeatures = ["prismaSchemaFolder"]
                }"#,
            ),
            (
                "datasource.prisma",
                r#"datasource db {
                    provider = "postgresql"
                    url = "postgresql://example.com/db"
                }"#,
            ),
        ];

        let request = json!({
            "prismaSchema": schemas,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client-js"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":["prismaSchemaFolder"]}],"datasources":[{"name":"db","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":null,"value":"postgresql://example.com/db"},"schemas":[]}],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_missing_env_var() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST","value":null},"schemas":[]}],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mEnvironment variable not found: NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:4\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 3 | \u001b[0m                provider = \"postgresql\"\n\u001b[1;94m 4 | \u001b[0m                url = \u001b[1;91menv(\"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_missing_env_var_with_ignore_env_var_error() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "ignoreEnvVarErrors": true,
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST","value":null},"schemas":[]}],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_with_env_vars() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "env": {
                "DBURL": "postgresql://example.com/mydb"
            }
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"},"schemas":[]}],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_direct_url_value() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = "postgresql://example.com/direct"
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "env": {
                "DBURL": "postgresql://example.com/mydb"
            }
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"},"directUrl":{"fromEnvVar":null,"value":"postgresql://example.com/direct"},"schemas":[]}],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_direct_url_env() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = env("DBDIRURL")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "env": {
                "DBURL": "postgresql://example.com/mydb",
                "DBDIRURL": "postgresql://example.com/direct"
            }
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"},"directUrl":{"fromEnvVar":"DBDIRURL","value":"postgresql://example.com/direct"},"schemas":[]}],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_direct_url_direct_empty() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = ""
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "env": {
                "DBURL": "postgresql://example.com/mydb",
            }
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":null},"directUrl":{"fromEnvVar":null,"value":""},"schemas":[]}],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating datasource `thedb`: You must provide a nonempty direct URL\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                directUrl = \u001b[1;91m\"\"\u001b[0m\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_direct_url_env_not_found() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = env("DOES_NOT_EXIST")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "env": {
                "DBURL": "postgresql://example.com/mydb",
            }
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":null},"directUrl":{"fromEnvVar":"DOES_NOT_EXIST","value":null},"schemas":[]}],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mEnvironment variable not found: DOES_NOT_EXIST.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                directUrl = \u001b[1;91menv(\"DOES_NOT_EXIST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_direct_url_env_is_empty() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = env("DOES_NOT_EXIST")
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
            "env": {
                "DBURL": "postgresql://example.com/mydb",
                "DOES_NOT_EXIST": "",
            }
        });
        let expected = expect![[
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":null},"directUrl":{"fromEnvVar":"DOES_NOT_EXIST","value":null},"schemas":[]}],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating datasource `thedb`: You must provide a nonempty direct URL. The environment variable `DOES_NOT_EXIST` resolved to an empty string.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                directUrl = \u001b[1;91menv(\"DOES_NOT_EXIST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }
}

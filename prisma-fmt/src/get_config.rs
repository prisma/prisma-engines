use psl::Diagnostics;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

use crate::validate::SCHEMA_PARSER_ERROR_CODE;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetConfigParams {
    prisma_schema: String,
    #[serde(default)]
    ignore_env_var_errors: bool,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    datasource_overrides: HashMap<String, String>,
}

#[derive(Debug)]
struct GetConfigError {
    error_code: Option<&'static str>,
    message: String,
}

pub(crate) fn get_config(params: &str) -> Result<String, String> {
    let params: GetConfigParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            panic!("Failed to deserialize GetConfigParams: {serde_err}",);
        }
    };

    get_config_impl(params)
        .map_err(|err| {
            json!({
                "message": err.message,
                "error_code": err.error_code,
            })
            .to_string()
        })
        .map(|value| value.to_string())
}

fn get_config_impl(params: GetConfigParams) -> Result<serde_json::Value, GetConfigError> {
    let wrap_get_config_err = |errors: Diagnostics| -> GetConfigError {
        use std::fmt::Write as _;

        let mut full_error = errors.to_pretty_string("schema.prisma", &params.prisma_schema);
        write!(full_error, "\nValidation Error Count: {}", errors.errors().len()).unwrap();

        GetConfigError {
            // this mirrors user_facing_errors::common::SchemaParserError
            error_code: Some(SCHEMA_PARSER_ERROR_CODE),
            message: full_error,
        }
    };

    let mut config = psl::parse_configuration(&params.prisma_schema).map_err(wrap_get_config_err)?;

    if !params.ignore_env_var_errors {
        let overrides: Vec<(_, _)> = params.datasource_overrides.into_iter().collect();
        config
            .resolve_datasource_urls_prisma_fmt(&overrides, |key| params.env.get(key).map(String::from))
            .map_err(wrap_get_config_err)?;
    }

    Ok(psl::get_config(&config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn get_config_invalid_schema() {
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
            r#"{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            \u001b[1;91mdatasøurce yolo {\u001b[0m\n\u001b[1;94m 6 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            datasøurce yolo {\n\u001b[1;94m 6 | \u001b[0m            \u001b[1;91m}\u001b[0m\n\u001b[1;94m 7 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n\u001b[1;91merror\u001b[0m: \u001b[1mArgument \"provider\" is missing in generator block \"js\".\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:2\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 1 | \u001b[0m\n\u001b[1;94m 2 | \u001b[0m            \u001b[1;91mgenerator js {\u001b[0m\n\u001b[1;94m 3 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 3","error_code":"P1012"}"#
        ]];

        let response = get_config(&request.to_string()).unwrap_err();
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
            r#"{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mEnvironment variable not found: NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:4\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 3 | \u001b[0m                provider = \"postgresql\"\n\u001b[1;94m 4 | \u001b[0m                url = \u001b[1;91menv(\"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1","error_code":"P1012"}"#
        ]];
        let response = get_config(&request.to_string()).unwrap_err();
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
            r#"{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST","value":null},"schemas":[]}],"warnings":[]}"#
        ]];
        let response = get_config(&request.to_string()).unwrap();
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
            r#"{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"},"schemas":[]}],"warnings":[]}"#
        ]];
        let response = get_config(&request.to_string()).unwrap();
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
            r#"{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"},"directUrl":{"fromEnvVar":null,"value":"postgresql://example.com/direct"},"schemas":[]}],"warnings":[]}"#
        ]];
        let response = get_config(&request.to_string()).unwrap();
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
            r#"{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"},"directUrl":{"fromEnvVar":"DBDIRURL","value":"postgresql://example.com/direct"},"schemas":[]}],"warnings":[]}"#
        ]];
        let response = get_config(&request.to_string()).unwrap();
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
            r#"{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating datasource `thedb`: You must provide a nonempty direct URL\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                directUrl = \u001b[1;91m\"\"\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1","error_code":"P1012"}"#
        ]];
        let response = get_config(&request.to_string()).unwrap_err();
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
            r#"{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mEnvironment variable not found: DOES_NOT_EXIST.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                directUrl = \u001b[1;91menv(\"DOES_NOT_EXIST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1","error_code":"P1012"}"#
        ]];
        let response = get_config(&request.to_string()).unwrap_err();
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
            r#"{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating datasource `thedb`: You must provide a nonempty direct URL. The environment variable `DOES_NOT_EXIST` resolved to an empty string.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                directUrl = \u001b[1;91menv(\"DOES_NOT_EXIST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1","error_code":"P1012"}"#
        ]];
        let response = get_config(&request.to_string()).unwrap_err();
        expected.assert_eq(&response);
    }
}

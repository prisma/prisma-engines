use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;

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
    error_code: Option<String>,
    message: String,
}

pub(crate) fn get_config(params: &str) -> String {
    let params: GetConfigParams = match serde_json::from_str(params) {
        Ok(params) => params,
        Err(serde_err) => {
            return json!({
                "error": {
                    "message": serde_err.to_string(),
                }
            })
            .to_string();
        }
    };

    match get_config_impl(params) {
        Err(err) => json!({
            "error": {
                "message": err.message,
                "error_code": err.error_code,
            }
        }),
        Ok(value) => value,
    }
    .to_string()
}

fn get_config_impl(params: GetConfigParams) -> Result<serde_json::Value, GetConfigError> {
    let mut config = match psl::parse_configuration(&params.prisma_schema) {
        Ok(config) => config,
        Err(err) => {
            return Err(GetConfigError {
                error_code: None,
                message: err.to_pretty_string("schema.prisma", &params.prisma_schema),
            })
        }
    };

    if !params.ignore_env_var_errors {
        let overrides: Vec<(_, _)> = params.datasource_overrides.into_iter().collect();
        config
            .resolve_datasource_urls_from_env(&overrides, |key| params.env.get(key).map(String::from))
            .map_err(|env_error| GetConfigError {
                error_code: None,
                message: env_error.to_pretty_string("schema.prisma", &params.prisma_schema),
            })?;
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
            r#"{"error":{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            \u001b[1;91mdatasøurce yolo {\u001b[0m\n\u001b[1;94m 6 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m            datasøurce yolo {\n\u001b[1;94m 6 | \u001b[0m            \u001b[1;91m}\u001b[0m\n\u001b[1;94m 7 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n\u001b[1;91merror\u001b[0m: \u001b[1mArgument \"provider\" is missing in generator block \"js\".\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:2\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 1 | \u001b[0m\n\u001b[1;94m 2 | \u001b[0m            \u001b[1;91mgenerator js {\u001b[0m\n\u001b[1;94m 3 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n","error_code":null}}"#
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
            r#"{"error":{"message":"\u001b[1;91merror\u001b[0m: \u001b[1mEnvironment variable not found: NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:4\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 3 | \u001b[0m                provider = \"postgresql\"\n\u001b[1;94m 4 | \u001b[0m                url = \u001b[1;91menv(\"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n","error_code":null}}"#
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
            r#"{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST","value":null}}],"warnings":[]}"#
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
            r#"{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","url":{"fromEnvVar":"DBURL","value":"postgresql://example.com/mydb"}}],"warnings":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }
}

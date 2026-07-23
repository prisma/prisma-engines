use psl::{diagnostics::DatamodelError, error_tolerant_parse_configuration, parser_database::Files};
use serde::{Deserialize, Serialize};

use crate::schema_file_input::SchemaFileInput;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetConfigParams {
    prisma_schema: SchemaFileInput,
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

    let (files, configuration, diagnostics) = error_tolerant_parse_configuration(&schema);

    let config = psl::get_config(&configuration, &files);

    let result = GetConfigResult {
        config,
        errors: serialize_errors(diagnostics.errors(), &files),
    };

    serde_json::to_string(&result).unwrap()
}

fn serialize_errors<'a>(
    errors: impl IntoIterator<Item = &'a DatamodelError>,
    files: &'a Files,
) -> Vec<ValidationError<'a>> {
    errors
        .into_iter()
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
                provider = "prisma-client"
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":[],"sourceFilePath":"schema.prisma"}],"datasources":[],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block_invalid_model() {
        let schema = r#"
            generator js {
                provider = "prisma-client"
            }

            model M {
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":[],"sourceFilePath":"schema.prisma"}],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m\n\u001b[1;94m 6 | \u001b[0m            \u001b[1;91mmodel M {\u001b[0m\n\u001b[1;94m 7 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block_invalid_model_field() {
        let schema = r#"
            generator js {
                provider = "prisma-client"
            }

            model M {
                field
            }
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":[],"sourceFilePath":"schema.prisma"}],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating model \"M\": This field declaration is invalid. It is either missing a name or a type.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:7\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 6 | \u001b[0m            model M {\n\u001b[1;94m 7 | \u001b[0m                \u001b[1;91mfield\u001b[0m\n\u001b[1;94m 8 | \u001b[0m            }\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn valid_generator_block_invalid_datasource() {
        let schema = r#"
            generator js {
                provider = "prisma-client"
            }

            datasource D {
        "#;

        let request = json!({
            "prismaSchema": schema,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":[],"sourceFilePath":"schema.prisma"}],"datasources":[],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m\n\u001b[1;94m 6 | \u001b[0m            \u001b[1;91mdatasource D {\u001b[0m\n\u001b[1;94m 7 | \u001b[0m        \n\u001b[1;94m   | \u001b[0m\n"}]}"#
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
                    provider = "prisma-client"
                }"#,
            ),
            (
                "datasource.prisma",
                r#"datasource db {
                    provider = "postgresql"
                }"#,
            ),
        ];

        let request = json!({
            "prismaSchema": schemas,
        });

        let expected = expect![[
            r#"{"config":{"generators":[{"name":"js","provider":{"fromEnvVar":null,"value":"prisma-client"},"output":null,"config":{},"binaryTargets":[],"previewFeatures":[],"sourceFilePath":"generator.prisma"}],"datasources":[{"name":"db","provider":"postgresql","activeProvider":"postgresql","schemas":[],"sourceFilePath":"datasource.prisma"}],"warnings":[]},"errors":[]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }

    #[test]
    fn get_config_urls_should_error() {
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
            r#"{"config":{"generators":[],"datasources":[{"name":"thedb","provider":"postgresql","activeProvider":"postgresql","schemas":[],"sourceFilePath":"schema.prisma"}],"warnings":[]},"errors":[{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mThe datasource property `url` is no longer supported in schema files. Move connection URLs for Migrate to `prisma.config.ts` and pass either `adapter` for a direct database connection or `accelerateUrl` for Accelerate to the `PrismaClient` constructor. See https://pris.ly/d/config-datasource and https://pris.ly/d/prisma7-client-config\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:4\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 3 | \u001b[0m                provider = \"postgresql\"\n\u001b[1;94m 4 | \u001b[0m                \u001b[1;91murl = env(\"DBURL\")\u001b[0m\n\u001b[1;94m   | \u001b[0m\n"},{"file_name":"schema.prisma","message":"\u001b[1;91merror\u001b[0m: \u001b[1mThe datasource property `directUrl` is no longer supported in schema files. Move connection URLs to `prisma.config.ts`. See https://pris.ly/d/config-datasource\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:5\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 4 | \u001b[0m                url = env(\"DBURL\")\n\u001b[1;94m 5 | \u001b[0m                \u001b[1;91mdirectUrl = \"postgresql://example.com/direct\"\u001b[0m\n\u001b[1;94m   | \u001b[0m\n"}]}"#
        ]];
        let response = get_config(&request.to_string());
        expected.assert_eq(&response);
    }
}

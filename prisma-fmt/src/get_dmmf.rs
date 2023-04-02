use dmmf::DataModelMetaFormat;
use serde::Deserialize;
use tsify::Tsify;

use crate::validate;

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
#[derive(Tsify)]
#[tsify(from_wasm_abi)]
pub struct GetDmmfParams {
    prisma_schema: String,
    #[serde(default)]
    no_color: bool,
}

pub(crate) fn get_dmmf(params: GetDmmfParams) -> Result<DataModelMetaFormat, String> {
    validate::run(&params.prisma_schema, params.no_color).map(|_| dmmf::dmmf_from_schema(&params.prisma_schema))
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;

    #[test]
    fn get_dmmf_invalid_schema_with_colors() {
        let schema = r#"
            generator js {
            }

            datasøurce yolo {
            }
        "#;

        let request = GetDmmfParams {
            prisma_schema: schema.to_string(),
            ..Default::default()
        };

        let expected = expect![[r#""{\"error_code\":\"P1012\",\"message\":\"\\u001b[1;91merror\\u001b[0m: \\u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\\u001b[0m\\n  \\u001b[1;94m-->\\u001b[0m  \\u001b[4mschema.prisma:5\\u001b[0m\\n\\u001b[1;94m   | \\u001b[0m\\n\\u001b[1;94m 4 | \\u001b[0m\\n\\u001b[1;94m 5 | \\u001b[0m            \\u001b[1;91mdatasøurce yolo {\\u001b[0m\\n\\u001b[1;94m 6 | \\u001b[0m            }\\n\\u001b[1;94m   | \\u001b[0m\\n\\u001b[1;91merror\\u001b[0m: \\u001b[1mError validating: This line is invalid. It does not start with any known Prisma schema keyword.\\u001b[0m\\n  \\u001b[1;94m-->\\u001b[0m  \\u001b[4mschema.prisma:6\\u001b[0m\\n\\u001b[1;94m   | \\u001b[0m\\n\\u001b[1;94m 5 | \\u001b[0m            datasøurce yolo {\\n\\u001b[1;94m 6 | \\u001b[0m            \\u001b[1;91m}\\u001b[0m\\n\\u001b[1;94m 7 | \\u001b[0m        \\n\\u001b[1;94m   | \\u001b[0m\\n\\u001b[1;91merror\\u001b[0m: \\u001b[1mArgument \\\"provider\\\" is missing in generator block \\\"js\\\".\\u001b[0m\\n  \\u001b[1;94m-->\\u001b[0m  \\u001b[4mschema.prisma:2\\u001b[0m\\n\\u001b[1;94m   | \\u001b[0m\\n\\u001b[1;94m 1 | \\u001b[0m\\n\\u001b[1;94m 2 | \\u001b[0m            \\u001b[1;91mgenerator js {\\u001b[0m\\n\\u001b[1;94m 3 | \\u001b[0m            }\\n\\u001b[1;94m   | \\u001b[0m\\n\\nValidation Error Count: 3\"}""#]];

        let dmmf = get_dmmf(request).unwrap_err();
        let dmmf_json = serde_json::to_string(&dmmf).unwrap();
        expected.assert_eq(&dmmf_json);
    }

    #[test]
    fn get_dmmf_missing_env_var() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("NON_EXISTING_ENV_VAR_WE_COUNT_ON_IT_AT_LEAST")
            }
        "#;

        let request = GetDmmfParams {
            prisma_schema: schema.to_string(),
            ..Default::default()
        };

        let expected = expect![[
            r#"{"datamodel":{"enums":[],"models":[],"types":[]},"schema":{"inputObjectTypes":{},"outputObjectTypes":{"prisma":[{"name":"Query","fields":[]},{"name":"Mutation","fields":[{"name":"executeRaw","args":[{"name":"query","isRequired":true,"isNullable":false,"inputTypes":[{"type":"String","location":"scalar","isList":false}]},{"name":"parameters","isRequired":false,"isNullable":false,"inputTypes":[{"type":"Json","location":"scalar","isList":false}]}],"isNullable":false,"outputType":{"type":"Json","location":"scalar","isList":false}},{"name":"queryRaw","args":[{"name":"query","isRequired":true,"isNullable":false,"inputTypes":[{"type":"String","location":"scalar","isList":false}]},{"name":"parameters","isRequired":false,"isNullable":false,"inputTypes":[{"type":"Json","location":"scalar","isList":false}]}],"isNullable":false,"outputType":{"type":"Json","location":"scalar","isList":false}}]}]},"enumTypes":{"prisma":[{"name":"TransactionIsolationLevel","values":["ReadUncommitted","ReadCommitted","RepeatableRead","Serializable"]}]},"fieldRefTypes":{}},"mappings":{"modelOperations":[],"otherOperations":{"read":[],"write":["executeRaw","queryRaw"]}}}"#
        ]];
        let dmmf = get_dmmf(request).unwrap();
        let dmmf_json = serde_json::to_string(&dmmf).unwrap();
        expected.assert_eq(&dmmf_json);
    }

    #[test]
    fn get_dmmf_direct_url_direct_empty() {
        let schema = r#"
            datasource thedb {
                provider = "postgresql"
                url = env("DBURL")
                directUrl = ""
            }
        "#;

        let request = GetDmmfParams {
            prisma_schema: schema.to_string(),
            ..Default::default()
        };

        let expected = expect![[
            r#"{"datamodel":{"enums":[],"models":[],"types":[]},"schema":{"inputObjectTypes":{},"outputObjectTypes":{"prisma":[{"name":"Query","fields":[]},{"name":"Mutation","fields":[{"name":"executeRaw","args":[{"name":"query","isRequired":true,"isNullable":false,"inputTypes":[{"type":"String","location":"scalar","isList":false}]},{"name":"parameters","isRequired":false,"isNullable":false,"inputTypes":[{"type":"Json","location":"scalar","isList":false}]}],"isNullable":false,"outputType":{"type":"Json","location":"scalar","isList":false}},{"name":"queryRaw","args":[{"name":"query","isRequired":true,"isNullable":false,"inputTypes":[{"type":"String","location":"scalar","isList":false}]},{"name":"parameters","isRequired":false,"isNullable":false,"inputTypes":[{"type":"Json","location":"scalar","isList":false}]}],"isNullable":false,"outputType":{"type":"Json","location":"scalar","isList":false}}]}]},"enumTypes":{"prisma":[{"name":"TransactionIsolationLevel","values":["ReadUncommitted","ReadCommitted","RepeatableRead","Serializable"]}]},"fieldRefTypes":{}},"mappings":{"modelOperations":[],"otherOperations":{"read":[],"write":["executeRaw","queryRaw"]}}}"#
        ]];
        let dmmf = get_dmmf(request).unwrap();
        let dmmf_json = serde_json::to_string(&dmmf).unwrap();
        expected.assert_eq(&dmmf_json);
    }

    #[test]
    fn get_dmmf_using_both_relation_mode_and_referential_integrity() {
        let schema = r#"
          datasource db {
              provider = "sqlite"
              url = "sqlite"
              relationMode = "prisma"
              referentialIntegrity = "foreignKeys"
          }
        "#;

        let request = GetDmmfParams {
            prisma_schema: schema.to_string(),
            ..Default::default()
        };

        let expected = expect![[
            r#"{"error_code":"P1012","message":"\u001b[1;91merror\u001b[0m: \u001b[1mThe `referentialIntegrity` and `relationMode` attributes cannot be used together. Please use only `relationMode` instead.\u001b[0m\n  \u001b[1;94m-->\u001b[0m  \u001b[4mschema.prisma:6\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\u001b[1;94m 5 | \u001b[0m              relationMode = \"prisma\"\n\u001b[1;94m 6 | \u001b[0m              \u001b[1;91mreferentialIntegrity = \"foreignKeys\"\u001b[0m\n\u001b[1;94m   | \u001b[0m\n\nValidation Error Count: 1"}"#
        ]];
        let dmmf_error = get_dmmf(request).unwrap_err();
        expected.assert_eq(&dmmf_error);
    }
}

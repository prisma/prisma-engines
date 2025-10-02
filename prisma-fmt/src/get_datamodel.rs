use serde::Deserialize;

use crate::{schema_file_input::SchemaFileInput, validate};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GetDatamodelParams {
    prisma_schema: SchemaFileInput,
    #[serde(default)]
    no_color: bool,
}

pub(crate) fn get_datamodel(params: &str) -> Result<String, String> {
    let params: GetDatamodelParams =
        serde_json::from_str(params).map_err(|e| format!("Failed to deserialize GetDatamodelParams: {e}"))?;

    let schema = validate::run(params.prisma_schema, params.no_color)?;

    let datamodel = dmmf::datamodel_from_validated_schema(&schema);

    Ok(serde_json::to_string(&datamodel).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use expect_test::expect;
    use indoc::indoc;
    use serde_json::json;

    #[test]
    fn sample_schema() {
        let schema = indoc! {r#"
            generator js {
                provider = "prisma-client"
            }

            datasource db {
                provider = "postgresql"
                url      = env("DATABASE_URL")
            }

            model User {
                id    Int    @id @default(autoincrement())
                email String @unique
                posts Post[]
            }

            model Post {
                id        Int     @id @default(autoincrement())
                title     String
                author    User    @relation(fields: [authorId], references: [id])
                authorId  Int

                @@index([title], name: "idx_post_on_title")
            }
        "#};

        let expected = expect![[r#"
            {
              "enums": [],
              "models": [
                {
                  "name": "User",
                  "dbName": null,
                  "schema": null,
                  "fields": [
                    {
                      "name": "id",
                      "kind": "scalar",
                      "isList": false,
                      "isRequired": true,
                      "isUnique": false,
                      "isId": true,
                      "isReadOnly": false,
                      "hasDefaultValue": true,
                      "type": "Int",
                      "nativeType": null,
                      "default": {
                        "name": "autoincrement",
                        "args": []
                      },
                      "isGenerated": false,
                      "isUpdatedAt": false
                    },
                    {
                      "name": "email",
                      "kind": "scalar",
                      "isList": false,
                      "isRequired": true,
                      "isUnique": true,
                      "isId": false,
                      "isReadOnly": false,
                      "hasDefaultValue": false,
                      "type": "String",
                      "nativeType": null,
                      "isGenerated": false,
                      "isUpdatedAt": false
                    },
                    {
                      "name": "posts",
                      "kind": "object",
                      "isList": true,
                      "isRequired": true,
                      "isUnique": false,
                      "isId": false,
                      "isReadOnly": false,
                      "hasDefaultValue": false,
                      "type": "Post",
                      "nativeType": null,
                      "relationName": "PostToUser",
                      "relationFromFields": [],
                      "relationToFields": [],
                      "isGenerated": false,
                      "isUpdatedAt": false
                    }
                  ],
                  "primaryKey": null,
                  "uniqueFields": [],
                  "uniqueIndexes": [],
                  "isGenerated": false
                },
                {
                  "name": "Post",
                  "dbName": null,
                  "schema": null,
                  "fields": [
                    {
                      "name": "id",
                      "kind": "scalar",
                      "isList": false,
                      "isRequired": true,
                      "isUnique": false,
                      "isId": true,
                      "isReadOnly": false,
                      "hasDefaultValue": true,
                      "type": "Int",
                      "nativeType": null,
                      "default": {
                        "name": "autoincrement",
                        "args": []
                      },
                      "isGenerated": false,
                      "isUpdatedAt": false
                    },
                    {
                      "name": "title",
                      "kind": "scalar",
                      "isList": false,
                      "isRequired": true,
                      "isUnique": false,
                      "isId": false,
                      "isReadOnly": false,
                      "hasDefaultValue": false,
                      "type": "String",
                      "nativeType": null,
                      "isGenerated": false,
                      "isUpdatedAt": false
                    },
                    {
                      "name": "author",
                      "kind": "object",
                      "isList": false,
                      "isRequired": true,
                      "isUnique": false,
                      "isId": false,
                      "isReadOnly": false,
                      "hasDefaultValue": false,
                      "type": "User",
                      "nativeType": null,
                      "relationName": "PostToUser",
                      "relationFromFields": [
                        "authorId"
                      ],
                      "relationToFields": [
                        "id"
                      ],
                      "isGenerated": false,
                      "isUpdatedAt": false
                    },
                    {
                      "name": "authorId",
                      "kind": "scalar",
                      "isList": false,
                      "isRequired": true,
                      "isUnique": false,
                      "isId": false,
                      "isReadOnly": true,
                      "hasDefaultValue": false,
                      "type": "Int",
                      "nativeType": null,
                      "isGenerated": false,
                      "isUpdatedAt": false
                    }
                  ],
                  "primaryKey": null,
                  "uniqueFields": [],
                  "uniqueIndexes": [],
                  "isGenerated": false
                }
              ],
              "types": [],
              "indexes": [
                {
                  "model": "User",
                  "type": "id",
                  "isDefinedOnField": true,
                  "fields": [
                    {
                      "name": "id"
                    }
                  ]
                },
                {
                  "model": "User",
                  "type": "unique",
                  "isDefinedOnField": true,
                  "fields": [
                    {
                      "name": "email"
                    }
                  ]
                },
                {
                  "model": "Post",
                  "type": "id",
                  "isDefinedOnField": true,
                  "fields": [
                    {
                      "name": "id"
                    }
                  ]
                },
                {
                  "model": "Post",
                  "type": "normal",
                  "isDefinedOnField": false,
                  "dbName": "idx_post_on_title",
                  "fields": [
                    {
                      "name": "title"
                    }
                  ]
                }
              ]
            }"#]];

        let response = get_datamodel(
            &json!({
                "prismaSchema": schema
            })
            .to_string(),
        )
        .unwrap();

        let prettified_response =
            serde_json::to_string_pretty(&serde_json::from_str::<serde_json::Value>(&response).unwrap()).unwrap();

        expected.assert_eq(&prettified_response);
    }
}

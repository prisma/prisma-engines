use crate::{FieldQuery, HandlerError, JsonSingleQuery, SelectionSet};
use bigdecimal::{BigDecimal, FromPrimitive};
use indexmap::IndexMap;
use prisma_models::{decode_bytes, parse_datetime, PrismaValue};
use query_core::{
    constants::custom_types,
    schema::{Identifier, ObjectTypeStrongRef, OutputFieldRef, QuerySchemaRef},
    Operation, Selection,
};
use serde_json::Value as JsonValue;
use std::{collections::HashSet, str::FromStr};

pub struct JsonProtocolAdapter;

impl JsonProtocolAdapter {
    pub fn convert_single(query: JsonSingleQuery, query_schema: &QuerySchemaRef) -> crate::Result<Operation> {
        let JsonSingleQuery {
            model_name,
            action,
            query,
        } = query;

        let (is_read, field) = {
            if let Some(field) =
                query_schema.find_query_field_by_model_and_action(model_name.as_deref(), action.value())
            {
                Ok((true, field))
            } else if let Some(field) =
                query_schema.find_mutation_field_by_model_and_action(model_name.as_deref(), action.value())
            {
                Ok((false, field))
            } else {
                Err(HandlerError::query_conversion(format!(
                    "Operation '{}' for model '{}' does not match any query.",
                    action.value(),
                    model_name.unwrap_or_else(|| "None".to_string())
                )))
            }
        }?;

        let selection = Self::convert_selection(&field, query)?;

        match is_read {
            true => Ok(Operation::Read(selection)),
            false => Ok(Operation::Write(selection)),
        }
    }

    fn convert_selection(field: &OutputFieldRef, query: FieldQuery) -> crate::Result<Selection> {
        let FieldQuery {
            arguments,
            selection: query_selection,
        } = query;
        let model = field.model();

        let arguments = match arguments {
            Some(args) => Self::convert_arguments(args)?,
            None => vec![],
        };
        let all_scalars_set = query_selection.all_scalars();
        let all_composites_set = query_selection.all_composites();

        let mut selection = Selection::new(&field.name, None, arguments, Vec::new());

        let json_selection = query_selection.selection();

        if !json_selection.is_empty() {
            let object_type = field.field_type.as_object_type().unwrap();

            for (selection_name, selected) in json_selection {
                match selected {
                    crate::SelectionSetValue::Shorthand(true) if SelectionSet::is_all_scalars(&selection_name) => {
                        Self::default_scalar_selection(&object_type, &mut selection);
                    }
                    crate::SelectionSetValue::Shorthand(true) if SelectionSet::is_all_composites(&selection_name) => {
                        Self::default_composite_selection(
                            field,
                            &object_type,
                            &mut selection,
                            true,
                            &mut HashSet::<Identifier>::new(),
                        )?;
                    }
                    crate::SelectionSetValue::Shorthand(true) => {
                        if all_scalars_set {
                            return Err(HandlerError::query_conversion(format!(
                                "Cannot select both all scalars and a specific scalar field '{}' for operation {}.",
                                selection_name, &field.name
                            )));
                        }

                        selection.push_nested_selection(Selection::with_name(selection_name));
                    }
                    crate::SelectionSetValue::Shorthand(false) => (),
                    crate::SelectionSetValue::Nested(nested) => {
                        let nested_field = object_type.find_field(&selection_name).ok_or_else(|| {
                            HandlerError::query_conversion(format!(
                                "Unknown nested field '{}' for operation {} does not match any query.",
                                selection_name, &field.name
                            ))
                        })?;

                        let is_composite_field = model
                            .and_then(|model| model.fields().find_from_composite(&nested_field.name).ok())
                            .is_some();

                        if is_composite_field && all_composites_set {
                            return Err(HandlerError::query_conversion(format!(
                                "Cannot select both all composites and a specific composite field '{}' for operation {}.",
                                selection_name, &field.name
                            )));
                        }

                        selection.push_nested_selection(Self::convert_selection(&nested_field, nested)?);
                    }
                }
            }
        }

        Ok(selection)
    }

    fn convert_arguments(args: IndexMap<String, JsonValue>) -> crate::Result<Vec<(String, PrismaValue)>> {
        let mut res = vec![];

        for (name, value) in args {
            let value = Self::convert_argument(value)?;

            res.push((name, value));
        }

        Ok(res)
    }

    fn convert_argument(value: JsonValue) -> crate::Result<PrismaValue> {
        let err_message = format!("Could not convert argument value {:?} to PrismaValue.", &value);
        let build_err = || HandlerError::query_conversion(err_message.clone());

        match value {
            serde_json::Value::String(s) => Ok(PrismaValue::String(s)),
            serde_json::Value::Array(v) => {
                let vals: crate::Result<Vec<PrismaValue>> = v.into_iter().map(Self::convert_argument).collect();

                Ok(PrismaValue::List(vals?))
            }
            serde_json::Value::Null => Ok(PrismaValue::Null),
            serde_json::Value::Bool(b) => Ok(PrismaValue::Boolean(b)),
            serde_json::Value::Number(num) => {
                if num.is_i64() {
                    Ok(PrismaValue::Int(num.as_i64().unwrap()))
                } else {
                    let fl = num.as_f64().unwrap();
                    let dec = BigDecimal::from_f64(fl).unwrap().normalized();

                    Ok(PrismaValue::Float(dec))
                }
            }
            serde_json::Value::Object(mut obj) => match obj.get(custom_types::TYPE).as_ref().and_then(|s| s.as_str()) {
                Some(custom_types::DATETIME) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;
                    let date = parse_datetime(value).map_err(|_| build_err())?;

                    Ok(PrismaValue::DateTime(date))
                }
                Some(custom_types::BIGINT) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    i64::from_str(value).map(PrismaValue::BigInt).map_err(|_| build_err())
                }
                Some(custom_types::DECIMAL) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    BigDecimal::from_str(value)
                        .map(PrismaValue::Float)
                        .map_err(|_| build_err())
                }
                Some(custom_types::BYTES) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    decode_bytes(value).map(PrismaValue::Bytes).map_err(|_| build_err())
                }
                Some(custom_types::JSON) => {
                    let value = obj
                        .remove(custom_types::VALUE)
                        .and_then(|v| match v {
                            JsonValue::String(str) => Some(str),
                            _ => None,
                        })
                        .ok_or_else(build_err)?;

                    Ok(PrismaValue::Json(value))
                }
                Some(custom_types::FIELD_REF) => {
                    let value = obj
                        .remove(custom_types::VALUE)
                        .and_then(|v| match v {
                            JsonValue::Object(obj) => Some(obj),
                            _ => None,
                        })
                        .ok_or_else(build_err)?;

                    Self::convert_argument(JsonValue::Object(value))
                }
                _ => {
                    let values = obj
                        .into_iter()
                        .map(|(k, v)| Ok((k, Self::convert_argument(v)?)))
                        .collect::<crate::Result<Vec<_>>>()?;

                    Ok(PrismaValue::Object(values))
                }
            },
        }
    }

    fn default_scalar_selection(object_type: &ObjectTypeStrongRef, selection: &mut Selection) {
        for scalar in object_type
            .get_fields()
            .iter()
            .filter(|f| f.field_type.is_scalar() || f.field_type.is_scalar_list())
        {
            selection.push_nested_selection(Selection::with_name(scalar.name.to_owned()));
        }
    }

    fn default_composite_selection(
        operation_field: &OutputFieldRef,
        object_type: &ObjectTypeStrongRef,
        selection: &mut Selection,
        parent_is_model: bool,
        walked_types: &mut HashSet<Identifier>,
    ) -> crate::Result<()> {
        let model = operation_field.model();

        // TODO: Figure out how to handle recursive types
        for field in object_type.get_fields() {
            // If we're traversing a composite type from another composite type
            // and it's a scalar/scalar list field, push it.
            if !parent_is_model && !field.field_type.is_object() {
                selection.push_nested_selection(Selection::with_name(field.name.to_owned()));
            } else {
                let is_composite_field = model
                    .and_then(|model| model.fields().find_from_composite(&field.name).ok())
                    .is_some();

                if parent_is_model && !is_composite_field {
                    continue;
                }

                let composite_type = field.field_type.as_object_type().unwrap();

                if walked_types.contains(composite_type.identifier()) {
                    return Err(HandlerError::query_conversion(
                        "$composites: true does not support recursive composite types.",
                    ));
                }

                walked_types.insert(composite_type.identifier().to_owned());

                let mut nested_selection = Selection::with_name(field.name.to_owned());

                Self::default_composite_selection(
                    operation_field,
                    &composite_type,
                    &mut nested_selection,
                    false,
                    walked_types,
                )?;

                selection.push_nested_selection(nested_selection);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use query_core::schema;
    use std::sync::Arc;

    fn schema() -> schema::QuerySchemaRef {
        let schema_str = r#"
          generator client {
            provider        = "prisma-client-js"
          }
          
          datasource db {
            provider = "mongodb"
            url      = "mongodb://"
          }
          
          model User {
            id String @id @map("_id")
            name String?
            email String @unique
            tags  String[]
            posts Post[]
            address Address
          }
          model Post {
            id String @id @map("_id")
            title String
            userId String 
            user User @relation(fields: [userId], references: [id])
          }

          type Address {
            number Int
            street String
            zipCode Int
          }
        "#;
        let mut schema = psl::validate(schema_str.into());

        schema.diagnostics.to_result().unwrap();

        let internal_data_model = prisma_models::convert(Arc::new(schema));

        Arc::new(schema_builder::build(internal_data_model, true))
    }

    #[test]
    pub fn default_scalar_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "findFirst",
            "query": {
                "selection": { "$scalars": true }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Read(
            Selection {
                name: "findFirstUser",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "id",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "name",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "email",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "tags",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                ],
            },
        )
        "###);
    }

    #[test]
    pub fn default_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "createOne",
            "query": {
                "selection": { "$composites": true }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Write(
            Selection {
                name: "createOneUser",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "address",
                        alias: None,
                        arguments: [],
                        nested_selections: [
                            Selection {
                                name: "number",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                            Selection {
                                name: "street",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                            Selection {
                                name: "zipCode",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                        ],
                    },
                ],
            },
        )
        "###);
    }

    #[test]
    pub fn explicit_select() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "findFirst",
            "query": {
                "selection": {
                    "id": true,
                    "email": false
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Read(
            Selection {
                name: "findFirstUser",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "id",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                ],
            },
        )
        "###);
    }

    #[test]
    pub fn arguments() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "findFirst",
            "query": {
                "arguments": {
                    "where": {
                        "id": "123"
                    }
                },
                "selection": { "$scalars": true }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Read(
            Selection {
                name: "findFirstUser",
                alias: None,
                arguments: [
                    (
                        "where",
                        Object(
                            [
                                (
                                    "id",
                                    String(
                                        "123",
                                    ),
                                ),
                            ],
                        ),
                    ),
                ],
                nested_selections: [
                    Selection {
                        name: "id",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "name",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "email",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "tags",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                ],
            },
        )
        "###);
    }

    #[test]
    pub fn nested_arguments() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "findFirst",
            "query": {
                "selection": {
                    "$scalars": true,
                    "posts": {
                        "arguments": {
                            "where": { "title": "something" }
                        },
                        "selection": { "$scalars": true }
                    }
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Read(
            Selection {
                name: "findFirstUser",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "id",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "name",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "email",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "tags",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "posts",
                        alias: None,
                        arguments: [
                            (
                                "where",
                                Object(
                                    [
                                        (
                                            "title",
                                            String(
                                                "something",
                                            ),
                                        ),
                                    ],
                                ),
                            ),
                        ],
                        nested_selections: [
                            Selection {
                                name: "id",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                            Selection {
                                name: "title",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                            Selection {
                                name: "userId",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                        ],
                    },
                ],
            },
        )
        "###);
    }

    #[test]
    pub fn mutation() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "selection": {
                    "id": true,
                    "email": false
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Write(
            Selection {
                name: "updateOneUser",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "id",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                ],
            },
        )
        "###);
    }

    #[test]
    pub fn scalar_wildcard_and_scalar_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "selection": {
                    "$scalars": true,
                    "id": true,
                    "email": false
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema());

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Cannot select both all scalars and a specific scalar field 'id' for operation updateOneUser.",
            ),
        )
        "###);
    }

    #[test]
    pub fn composite_wildcard_and_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "selection": {
                    "$composites": true,
                    "address": {
                        "selection": {
                            "$scalars": true
                        }
                    }
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema());

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Cannot select both all composites and a specific composite field 'address' for operation updateOneUser.",
            ),
        )
        "###);
    }

    #[test]
    pub fn composite_wildcard_and_scalar_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "selection": {
                    "$composites": true,
                    "id": true,
                    "email": false
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema());

        assert_debug_snapshot!(operation, @r###"
        Ok(
            Write(
                Selection {
                    name: "updateOneUser",
                    alias: None,
                    arguments: [],
                    nested_selections: [
                        Selection {
                            name: "address",
                            alias: None,
                            arguments: [],
                            nested_selections: [
                                Selection {
                                    name: "number",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                                Selection {
                                    name: "street",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                                Selection {
                                    name: "zipCode",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                            ],
                        },
                        Selection {
                            name: "id",
                            alias: None,
                            arguments: [],
                            nested_selections: [],
                        },
                    ],
                },
            ),
        )
        "###);
    }

    #[test]
    pub fn custom_arg_datetime() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "DateTime", "value": "1900-10-10T01:10:10.001Z" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation.arguments()[0].1, @r###"
        Object(
            [
                (
                    "x",
                    DateTime(
                        1900-10-10T01:10:10.001+00:00,
                    ),
                ),
            ],
        )
        "###);
    }

    #[test]
    pub fn custom_arg_bigint() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "BigInt", "value": "9223372036854775807" },
                        "y": { "$type": "BigInt", "value": "-9223372036854775808" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation.arguments(), @r###"
        [
            (
                "data",
                Object(
                    [
                        (
                            "x",
                            BigInt(
                                9223372036854775807,
                            ),
                        ),
                        (
                            "y",
                            BigInt(
                                -9223372036854775808,
                            ),
                        ),
                    ],
                ),
            ),
        ]
        "###);
    }

    #[test]
    pub fn custom_arg_decimal() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "Decimal", "value": "123.45678910" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation.arguments()[0].1, @r###"
        Object(
            [
                (
                    "x",
                    Float(
                        BigDecimal("123.45678910"),
                    ),
                ),
            ],
        )
        "###);
    }

    #[test]
    pub fn custom_arg_bytes() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "Bytes", "value": "AQIDBA==" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation.arguments()[0].1, @r###"
        Object(
            [
                (
                    "x",
                    Bytes(
                        [
                            1,
                            2,
                            3,
                            4,
                        ],
                    ),
                ),
            ],
        )
        "###);
    }

    #[test]
    pub fn custom_arg_json() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "Json", "value": "{ \"$type\": \"foo\", \"value\": \"bar\" }" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation.arguments()[0].1, @r###"
        Object(
            [
                (
                    "x",
                    Json(
                        "{ \"$type\": \"foo\", \"value\": \"bar\" }",
                    ),
                ),
            ],
        )
        "###);
    }

    #[test]
    pub fn unknown_custom_arg() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "updateOne",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "Invalid", "value": "nope" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema()).unwrap();

        assert_debug_snapshot!(operation.arguments()[0].1, @r###"
        Object(
            [
                (
                    "x",
                    Object(
                        [
                            (
                                "$type",
                                String(
                                    "Invalid",
                                ),
                            ),
                            (
                                "value",
                                String(
                                    "nope",
                                ),
                            ),
                        ],
                    ),
                ),
            ],
        )
        "###);
    }

    #[test]
    pub fn invalid_operation() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "modelName": "User",
            "action": "queryRaw",
            "query": {
                "arguments": {
                    "data": {
                        "x": { "$type": "Invalid", "value": "nope" }
                    }
                },
                "selection": {
                    "$scalars": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &schema());

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Operation 'queryRaw' for model 'User' does not match any query.",
            ),
        )
        "###);
    }
}

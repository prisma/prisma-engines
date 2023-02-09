use crate::{FieldQuery, HandlerError, JsonSingleQuery, SelectionSet};
use bigdecimal::{BigDecimal, FromPrimitive};
use indexmap::IndexMap;
use prisma_models::{decode_bytes, parse_datetime, prelude::ParentContainer, Field};
use query_core::{
    constants::custom_types,
    schema::{ObjectTypeStrongRef, OutputFieldRef, QuerySchemaRef},
    ArgumentValue, Operation, Selection,
};
use serde_json::Value as JsonValue;
use std::{collections::HashSet, str::FromStr};

enum OperationType {
    Read,
    Write,
}

pub struct JsonProtocolAdapter;

impl JsonProtocolAdapter {
    pub fn convert_single(query: JsonSingleQuery, query_schema: &QuerySchemaRef) -> crate::Result<Operation> {
        let JsonSingleQuery {
            model_name,
            action,
            query,
        } = query;

        let (operation_type, field) = Self::find_schema_field(query_schema, model_name, action)?;
        let container = field.model().map(ParentContainer::from);

        let selection = Self::convert_selection(&field, container.as_ref(), query)?;

        match operation_type {
            OperationType::Read => Ok(Operation::Read(selection)),
            OperationType::Write => Ok(Operation::Write(selection)),
        }
    }

    fn convert_selection(
        field: &OutputFieldRef,
        container: Option<&ParentContainer>,
        query: FieldQuery,
    ) -> crate::Result<Selection> {
        let FieldQuery {
            arguments,
            selection: query_selection,
        } = query;

        let arguments = match arguments {
            Some(args) => Self::convert_arguments(args)?,
            None => vec![],
        };

        let all_scalars_set = query_selection.all_scalars();
        let all_composites_set = query_selection.all_composites();

        let mut selection = Selection::new(&field.name, None, arguments, Vec::new());

        if let Some(object_type) = field.field_type.as_object_type() {
            let json_selection = query_selection.selection();

            for (selection_name, selected) in json_selection {
                match selected {
                    // $scalars: true
                    crate::SelectionSetValue::Shorthand(true) if SelectionSet::is_all_scalars(&selection_name) => {
                        Self::default_scalar_selection(&object_type, &mut selection);
                    }
                    // $composites: true
                    crate::SelectionSetValue::Shorthand(true) if SelectionSet::is_all_composites(&selection_name) => {
                        if let Some(container) = container {
                            Self::default_composite_selection(
                                &mut selection,
                                container,
                                &mut HashSet::<String>::new(),
                            )?;
                        }
                    }
                    // <field_name>: true
                    crate::SelectionSetValue::Shorthand(true) => {
                        if all_scalars_set {
                            return Err(HandlerError::query_conversion(format!(
                                "Cannot select both '$scalars: true' and a specific scalar field '{selection_name}'.",
                            )));
                        }

                        selection.push_nested_selection(Selection::with_name(selection_name));
                    }
                    // <field_name>: false
                    crate::SelectionSetValue::Shorthand(false) => (),
                    // <field_name>: { query: { ... }, arguments: { ... } }
                    crate::SelectionSetValue::Nested(nested_query) => {
                        let nested_field = object_type.find_field(&selection_name).ok_or_else(|| {
                            HandlerError::query_conversion(format!(
                                "Unknown nested field '{}' for operation {} does not match any query.",
                                selection_name, &field.name
                            ))
                        })?;

                        let field = container.and_then(|container| container.find_field(&nested_field.name));
                        let is_composite_field = field.as_ref().map(|f| f.is_composite()).unwrap_or(false);
                        let nested_container = field.map(|f| match f {
                            Field::Relation(rf) => ParentContainer::from(rf.related_model()),
                            Field::Scalar(sf) => sf.container().clone(),
                            Field::Composite(cf) => ParentContainer::from(&cf.typ),
                        });

                        if is_composite_field && all_composites_set {
                            return Err(HandlerError::query_conversion(format!(
                                "Cannot select both '$composites: true' and a specific composite field '{selection_name}'.",
                            )));
                        }

                        selection.push_nested_selection(Self::convert_selection(
                            &nested_field,
                            nested_container.as_ref(),
                            nested_query,
                        )?);
                    }
                }
            }
        }

        Ok(selection)
    }

    fn convert_arguments(args: IndexMap<String, JsonValue>) -> crate::Result<Vec<(String, ArgumentValue)>> {
        let mut res = vec![];

        for (name, value) in args {
            let value = Self::convert_argument(value)?;

            res.push((name, value));
        }

        Ok(res)
    }

    fn convert_argument(value: JsonValue) -> crate::Result<ArgumentValue> {
        let err_message = format!("Could not convert argument value {:?} to ArgumentValue.", &value);
        let build_err = || HandlerError::query_conversion(err_message.clone());

        match value {
            serde_json::Value::String(s) => Ok(ArgumentValue::string(s)),
            serde_json::Value::Array(v) => {
                let vals: crate::Result<Vec<ArgumentValue>> = v.into_iter().map(Self::convert_argument).collect();

                Ok(ArgumentValue::List(vals?))
            }
            serde_json::Value::Null => Ok(ArgumentValue::null()),
            serde_json::Value::Bool(b) => Ok(ArgumentValue::bool(b)),
            serde_json::Value::Number(num) => {
                if num.is_i64() {
                    Ok(ArgumentValue::int(num.as_i64().unwrap()))
                } else {
                    let fl = num.as_f64().unwrap();
                    let dec = BigDecimal::from_f64(fl).unwrap().normalized();

                    Ok(ArgumentValue::float(dec))
                }
            }
            serde_json::Value::Object(mut obj) => match obj.get(custom_types::TYPE).as_ref().and_then(|s| s.as_str()) {
                Some(custom_types::DATETIME) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;
                    let date = parse_datetime(value).map_err(|_| build_err())?;

                    Ok(ArgumentValue::datetime(date))
                }
                Some(custom_types::BIGINT) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    i64::from_str(value).map(ArgumentValue::bigint).map_err(|_| build_err())
                }
                Some(custom_types::DECIMAL) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    BigDecimal::from_str(value)
                        .map(ArgumentValue::float)
                        .map_err(|_| build_err())
                }
                Some(custom_types::BYTES) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    decode_bytes(value).map(ArgumentValue::bytes).map_err(|_| build_err())
                }
                Some(custom_types::JSON) => {
                    let value = obj
                        .remove(custom_types::VALUE)
                        .and_then(|v| match v {
                            JsonValue::String(str) => Some(str),
                            _ => None,
                        })
                        .ok_or_else(build_err)?;

                    Ok(ArgumentValue::json(value))
                }
                Some(custom_types::ENUM) => {
                    let value = obj
                        .get(custom_types::VALUE)
                        .and_then(|v| v.as_str())
                        .ok_or_else(build_err)?;

                    Ok(ArgumentValue::r#enum(value.to_string()))
                }
                Some(custom_types::FIELD_REF) => {
                    let value = obj
                        .remove(custom_types::VALUE)
                        .and_then(|v| match v {
                            JsonValue::Object(obj) => Some(obj),
                            _ => None,
                        })
                        .ok_or_else(build_err)?;
                    let values = value
                        .into_iter()
                        .map(|(k, v)| Ok((k, Self::convert_argument(v)?)))
                        .collect::<crate::Result<IndexMap<_, _>>>()?;

                    Ok(ArgumentValue::FieldRef(values))
                }
                _ => {
                    let values = obj
                        .into_iter()
                        .map(|(k, v)| Ok((k, Self::convert_argument(v)?)))
                        .collect::<crate::Result<IndexMap<_, _>>>()?;

                    Ok(ArgumentValue::Object(values))
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
        selection: &mut Selection,
        container: &ParentContainer,
        walked_types: &mut HashSet<String>,
    ) -> crate::Result<()> {
        match container {
            ParentContainer::Model(model) => {
                let model = model.upgrade().unwrap();

                for cf in model.fields().composite() {
                    let mut nested_selection = Selection::with_name(cf.name());

                    Self::default_composite_selection(
                        &mut nested_selection,
                        &ParentContainer::from(&cf.typ),
                        walked_types,
                    )?;

                    selection.push_nested_selection(nested_selection);
                }
            }
            ParentContainer::CompositeType(ct) => {
                let ct = ct.upgrade().unwrap();

                if walked_types.contains(&ct.name) {
                    return Err(HandlerError::query_conversion(
                        "$composites: true does not support recursive composite types.",
                    ));
                }

                walked_types.insert(ct.name.to_owned());

                for f in ct.fields() {
                    match f {
                        Field::Scalar(s) => selection.push_nested_selection(Selection::with_name(s.name().to_owned())),
                        Field::Composite(cf) => {
                            let mut nested_selection = Selection::with_name(cf.name().to_owned());

                            Self::default_composite_selection(
                                &mut nested_selection,
                                &ParentContainer::from(&cf.typ),
                                walked_types,
                            )?;

                            selection.push_nested_selection(nested_selection);
                        }
                        Field::Relation(_) => unreachable!(),
                    }
                }
            }
        }

        Ok(())
    }

    fn find_schema_field(
        query_schema: &QuerySchemaRef,
        model_name: Option<String>,
        action: crate::Action,
    ) -> crate::Result<(OperationType, OutputFieldRef)> {
        if let Some(field) = query_schema.find_query_field_by_model_and_action(model_name.as_deref(), action.value()) {
            return Ok((OperationType::Read, field));
        };

        if let Some(field) = query_schema.find_mutation_field_by_model_and_action(model_name.as_deref(), action.value())
        {
            return Ok((OperationType::Write, field));
        };

        Err(HandlerError::query_conversion(format!(
            "Operation '{}' for model '{}' does not match any query.",
            action.value(),
            model_name.unwrap_or_else(|| "None".to_string())
        )))
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
                            {
                                "id": Scalar(
                                    String(
                                        "123",
                                    ),
                                ),
                            },
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
                                    {
                                        "title": Scalar(
                                            String(
                                                "something",
                                            ),
                                        ),
                                    },
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
                "Cannot select both '$scalars: true' and a specific scalar field 'id'.",
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
                "Cannot select both '$composites: true' and a specific composite field 'address'.",
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
            {
                "x": Scalar(
                    DateTime(
                        1900-10-10T01:10:10.001+00:00,
                    ),
                ),
            },
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
                    {
                        "x": Scalar(
                            BigInt(
                                9223372036854775807,
                            ),
                        ),
                        "y": Scalar(
                            BigInt(
                                -9223372036854775808,
                            ),
                        ),
                    },
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
            {
                "x": Scalar(
                    Float(
                        BigDecimal("123.45678910"),
                    ),
                ),
            },
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
            {
                "x": Scalar(
                    Bytes(
                        [
                            1,
                            2,
                            3,
                            4,
                        ],
                    ),
                ),
            },
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
            {
                "x": Scalar(
                    Json(
                        "{ \"$type\": \"foo\", \"value\": \"bar\" }",
                    ),
                ),
            },
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
            {
                "x": Object(
                    {
                        "$type": Scalar(
                            String(
                                "Invalid",
                            ),
                        ),
                        "value": Scalar(
                            String(
                                "nope",
                            ),
                        ),
                    },
                ),
            },
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
                        "x": "y"
                    }
                },
                "selection": {}
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

    #[test]
    pub fn query_raw() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
            "action": "runCommandRaw",
            "query": {
                "arguments": {
                    "data": {
                        "x": "y"
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
        Ok(
            Write(
                Selection {
                    name: "runCommandRaw",
                    alias: None,
                    arguments: [
                        (
                            "data",
                            Object(
                                {
                                    "x": Scalar(
                                        String(
                                            "y",
                                        ),
                                    ),
                                },
                            ),
                        ),
                    ],
                    nested_selections: [],
                },
            ),
        )
        "###);
    }

    fn composite_schema() -> schema::QuerySchemaRef {
        let schema_str = r#"
          generator client {
            provider        = "prisma-client-js"
          }
          
          datasource db {
            provider = "mongodb"
            url      = "mongodb://"
          }
          
          model Comment {
            id String @id @default(auto()) @map("_id") @db.ObjectId
          
            country String?
            content CommentContent
          }
          
          type CommentContent {
            text    String
            upvotes CommentContentUpvotes[]
          }
          
          type CommentContentUpvotes {
            vote   Boolean
            userId String
          }          
        "#;
        let mut schema = psl::validate(schema_str.into());

        schema.diagnostics.to_result().unwrap();

        let internal_data_model = prisma_models::convert(Arc::new(schema));

        Arc::new(schema_builder::build(internal_data_model, true))
    }

    #[test]
    fn nested_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"
            {
              "modelName": "Comment",
              "action": "createOne",
              "query": {
                "selection": {
                  "$scalars": true,
                  "$composites": true
                }
              }
            }"#,
        )
        .unwrap();

        let selection = JsonProtocolAdapter::convert_single(query, &composite_schema())
            .unwrap()
            .into_selection();

        assert_debug_snapshot!(selection.nested_selections(), @r###"
        [
            Selection {
                name: "id",
                alias: None,
                arguments: [],
                nested_selections: [],
            },
            Selection {
                name: "country",
                alias: None,
                arguments: [],
                nested_selections: [],
            },
            Selection {
                name: "content",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "text",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "upvotes",
                        alias: None,
                        arguments: [],
                        nested_selections: [
                            Selection {
                                name: "vote",
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
        ]
        "###);
    }

    #[test]
    pub fn nested_composite_wildcard_and_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            &r#"{
                "modelName": "Comment",
                "action": "createOne",
                "query": {
                  "selection": {
                    "content": {
                        "selection": {
                            "$composites": true,
                            "upvotes": {
                                "selection": { "vote": true }
                            }
                        }
                    }
                  }
                }
              }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::convert_single(query, &composite_schema());

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Cannot select both '$composites: true' and a specific composite field 'upvotes'.",
            ),
        )
        "###);
    }
}

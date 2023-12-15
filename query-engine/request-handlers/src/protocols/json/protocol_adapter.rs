use crate::{FieldQuery, HandlerError, JsonSingleQuery, SelectionSet};
use bigdecimal::{BigDecimal, FromPrimitive};
use indexmap::IndexMap;
use query_core::{
    constants::custom_types,
    schema::{ObjectType, OutputField, QuerySchema},
    ArgumentValue, Operation, Selection,
};
use query_structure::{decode_bytes, parse_datetime, prelude::ParentContainer, Field};
use serde_json::Value as JsonValue;
use std::str::FromStr;

enum OperationType {
    Read,
    Write,
}

pub struct JsonProtocolAdapter<'a> {
    query_schema: &'a QuerySchema,
}

impl<'a> JsonProtocolAdapter<'a> {
    pub fn new(query_schema: &'a QuerySchema) -> Self {
        JsonProtocolAdapter { query_schema }
    }

    pub fn convert_single(&mut self, query: JsonSingleQuery) -> crate::Result<Operation> {
        let JsonSingleQuery {
            model_name,
            action,
            query,
        } = query;

        let (operation_type, field) = self.find_schema_field(model_name, action)?;
        let container = field
            .model()
            .map(|id| self.query_schema.internal_data_model.clone().zip(id))
            .map(ParentContainer::from);

        let selection = self.convert_selection(&field, container.as_ref(), query)?;

        match operation_type {
            OperationType::Read => Ok(Operation::Read(selection)),
            OperationType::Write => Ok(Operation::Write(selection)),
        }
    }

    fn convert_selection(
        &mut self,
        field: &OutputField<'a>,
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

        let mut selection = Selection::new(field.name().clone(), None, arguments, Vec::new());

        for (selection_name, selected) in query_selection.into_selection() {
            match selected {
                // $scalars: true
                crate::SelectionSetValue::Shorthand(true) if SelectionSet::is_all_scalars(&selection_name) => {
                    if let Some(schema_object) = field.field_type().as_object_type() {
                        Self::default_scalar_selection(schema_object, &mut selection);
                    }
                }
                // $composites: true
                crate::SelectionSetValue::Shorthand(true) if SelectionSet::is_all_composites(&selection_name) => {
                    if let Some(schema_object) = field.field_type().as_object_type() {
                        if let Some(container) = container {
                            Self::default_composite_selection(
                                &mut selection,
                                container,
                                schema_object,
                                &mut Vec::<String>::new(),
                            )?;
                        }
                    }
                }
                // <field_name>: true
                crate::SelectionSetValue::Shorthand(true) => {
                    selection.push_nested_selection(self.create_shorthand_selection(
                        field,
                        &selection_name,
                        container,
                        all_scalars_set,
                    )?);
                }
                // <field_name>: false
                crate::SelectionSetValue::Shorthand(false) => (),
                // <field_name>: { selection: { ... }, arguments: { ... } }
                crate::SelectionSetValue::Nested(nested_query) => {
                    if field.field_type().as_object_type().is_some() {
                        let schema_field = field
                            .field_type()
                            .as_object_type()
                            .and_then(|t| t.find_field(selection_name.as_str()))
                            .ok_or_else(|| {
                                HandlerError::query_conversion(format!(
                                    "Unknown nested field '{}' for operation {} does not match any query.",
                                    selection_name,
                                    field.name()
                                ))
                            })?;

                        let field = container.and_then(|container| container.find_field(schema_field.name()));
                        let is_composite_field = field.as_ref().map(|f| f.is_composite()).unwrap_or(false);
                        let nested_container = field.map(|f| f.related_container());

                        if is_composite_field && all_composites_set {
                            return Err(HandlerError::query_conversion(format!(
                                "Cannot select both '$composites: true' and a specific composite field '{selection_name}'.",
                            )));
                        }

                        selection.push_nested_selection(self.convert_selection(
                            schema_field,
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

    fn create_shorthand_selection(
        &mut self,
        parent_field: &OutputField<'a>,
        nested_field_name: &str,
        container: Option<&ParentContainer>,
        all_scalars_set: bool,
    ) -> crate::Result<Selection> {
        let nested_object_type = parent_field
            .field_type()
            .as_object_type()
            .and_then(|parent_object| self.get_output_field(parent_object, nested_field_name))
            .and_then(|nested_field| nested_field.field_type.as_object_type());

        if let Some(nested_object_type) = nested_object_type {
            // case for a relation - we select all nested scalar fields and composite fields
            let mut nested_selection = Selection::new(nested_field_name, None, vec![], vec![]);
            let nested_container = container
                .and_then(|c| c.find_field(nested_field_name))
                .map(|f| f.related_container());

            Self::default_scalar_and_composite_selection(
                &mut nested_selection,
                nested_object_type,
                nested_container.as_ref(),
            )?;

            return Ok(nested_selection);
        }

        // case for a scalar - just picking the specified field without any nested selections
        if all_scalars_set {
            return Err(HandlerError::query_conversion(format!(
                "Cannot select both '$scalars: true' and a specific scalar field '{nested_field_name}'.",
            )));
        }

        Ok(Selection::with_name(nested_field_name))
    }

    fn default_scalar_selection(schema_object: &ObjectType, selection: &mut Selection) {
        for scalar in schema_object.get_fields().iter().filter(|f| {
            f.field_type().is_scalar()
                || f.field_type().is_scalar_list()
                || f.field_type().is_enum()
                || f.field_type().is_enum_list()
        }) {
            selection.push_nested_selection(Selection::with_name(scalar.name().clone()));
        }
    }

    fn default_composite_selection(
        selection: &mut Selection,
        container: &ParentContainer,
        schema_object: &ObjectType,
        walked_types_stack: &mut Vec<String>,
    ) -> crate::Result<()> {
        match container {
            ParentContainer::Model(model) => {
                for cf in model.fields().composite() {
                    let schema_field = schema_object.find_field(cf.name());

                    if let Some(schema_field) = schema_field {
                        let mut nested_selection = Selection::with_name(cf.name());

                        Self::default_composite_selection(
                            &mut nested_selection,
                            &ParentContainer::from(cf.typ()),
                            schema_field.field_type.as_object_type().unwrap(),
                            walked_types_stack,
                        )?;

                        selection.push_nested_selection(nested_selection);
                    }
                }
            }
            ParentContainer::CompositeType(ct) => {
                let composite_type_name = ct.name().to_owned();
                if walked_types_stack.contains(&composite_type_name) {
                    return Err(HandlerError::query_conversion(
                        "$composites: true does not support recursive composite types.",
                    ));
                }
                walked_types_stack.push(composite_type_name);
                for f in ct.fields() {
                    let field_name = f.name().to_owned();

                    let schema_field = schema_object.find_field(&field_name);

                    if let Some(schema_field) = schema_field {
                        match f {
                            Field::Scalar(s) => {
                                selection.push_nested_selection(Selection::with_name(s.name().to_owned()))
                            }
                            Field::Composite(cf) => {
                                let mut nested_selection = Selection::with_name(cf.name().to_owned());

                                Self::default_composite_selection(
                                    &mut nested_selection,
                                    &ParentContainer::from(cf.typ()),
                                    schema_field.field_type.as_object_type().unwrap(),
                                    walked_types_stack,
                                )?;

                                selection.push_nested_selection(nested_selection);
                            }
                            Field::Relation(_) => unreachable!(),
                        }
                    }
                }
                let _ = walked_types_stack.pop();
            }
        }

        Ok(())
    }

    fn default_scalar_and_composite_selection(
        selection: &mut Selection,
        schema_object: &ObjectType,
        container: Option<&ParentContainer>,
    ) -> crate::Result<()> {
        Self::default_scalar_selection(schema_object, selection);
        if let Some(container) = container {
            Self::default_composite_selection(selection, container, schema_object, &mut Vec::<String>::new())?;
        }

        Ok(())
    }

    fn find_schema_field(
        &mut self,
        model_name: Option<String>,
        action: crate::Action,
    ) -> crate::Result<(OperationType, OutputField<'a>)> {
        if let Some(field) = self
            .query_schema
            .find_query_field_by_model_and_action(model_name.as_deref(), action.value())
        {
            return Ok((OperationType::Read, field));
        };

        if let Some(field) = self
            .query_schema
            .find_mutation_field_by_model_and_action(model_name.as_deref(), action.value())
        {
            return Ok((OperationType::Write, field));
        };

        Err(HandlerError::query_conversion(format!(
            "Operation '{}' for model '{}' does not match any query.",
            action.value(),
            model_name.unwrap_or_else(|| "None".to_string())
        )))
    }

    fn get_output_field<'b>(&mut self, ty: &'b ObjectType<'a>, name: &str) -> Option<&'b OutputField<'a>> {
        ty.find_field(name)
    }
}

#[cfg(test)]
#[allow(clippy::needless_raw_string_hashes)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use query_core::schema;
    use std::sync::Arc;

    fn schema() -> schema::QuerySchema {
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
            role Role
            roles Role[]
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

          enum Role {
            Admin
            User
          }
        "#;
        let mut schema = psl::validate(schema_str.into());

        schema.diagnostics.to_result().unwrap();

        schema::build(Arc::new(schema), true)
    }

    #[test]
    pub fn default_scalar_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
            "modelName": "User",
            "action": "findFirst",
            "query": {
                "selection": { "$scalars": true }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
                        name: "role",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "roles",
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
    fn default_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
            "modelName": "User",
            "action": "createOne",
            "query": {
                "selection": { "$composites": true }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn explicit_select() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn relation_shorthand() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
            "modelName": "Post",
            "action": "findFirst",
            "query": {
                "selection": {
                    "user": true
                }
            }
        }"#,
        )
        .unwrap();
        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();
        assert_debug_snapshot!(operation, @r###"
        Read(
            Selection {
                name: "findFirstPost",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "user",
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
                                name: "role",
                                alias: None,
                                arguments: [],
                                nested_selections: [],
                            },
                            Selection {
                                name: "roles",
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
                ],
            },
        )
        "###);
    }

    #[test]
    fn arguments() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
                        name: "role",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "roles",
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
    fn nested_arguments() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
                        name: "role",
                        alias: None,
                        arguments: [],
                        nested_selections: [],
                    },
                    Selection {
                        name: "roles",
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
    fn composite_selection_should_be_based_on_schema_1() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
            "modelName": "User",
            "action": "deleteMany",
            "query": {
                "selection": {
                    "$scalars": true,
                    "$composites": true
                }
            }
        }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

        assert_debug_snapshot!(operation, @r###"
        Write(
            Selection {
                name: "deleteManyUser",
                alias: None,
                arguments: [],
                nested_selections: [
                    Selection {
                        name: "count",
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
    fn simple_mutation() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn scalar_wildcard_and_scalar_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Cannot select both '$scalars: true' and a specific scalar field 'id'.",
            ),
        )
        "###);
    }

    #[test]
    fn composite_wildcard_and_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Cannot select both '$composites: true' and a specific composite field 'address'.",
            ),
        )
        "###);
    }

    #[test]
    fn composite_wildcard_and_scalar_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query);

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
    fn custom_arg_datetime() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn custom_arg_bigint() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn custom_arg_decimal() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn custom_arg_bytes() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn custom_arg_json() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn unknown_custom_arg() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query).unwrap();

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
    fn invalid_operation() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Operation 'queryRaw' for model 'User' does not match any query.",
            ),
        )
        "###);
    }

    #[test]
    fn query_raw() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&schema()).convert_single(query);

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

    fn composite_schema() -> schema::QuerySchema {
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

        schema::build(Arc::new(schema), true)
    }

    #[test]
    fn nested_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"
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

        let selection = JsonProtocolAdapter::new(&composite_schema())
            .convert_single(query)
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
    fn nested_composite_wildcard_and_composite_selection() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
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

        let operation = JsonProtocolAdapter::new(&composite_schema()).convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "Cannot select both '$composites: true' and a specific composite field 'upvotes'.",
            ),
        )
        "###);
    }

    fn recursive_composite_schema() -> schema::QuerySchema {
        let schema_str = r#"
          generator client {
            provider        = "prisma-client-js"
          }
          
          datasource db {
            provider = "mongodb"
            url      = "mongodb://"
          }
          
          model List {
            id String @id @default(auto()) @map("_id") @db.ObjectId
            head ListNode?
          }
          
          type ListNode  {
            value Int
            next ListNode? 
          }        
        "#;
        let mut schema = psl::validate(schema_str.into());

        schema.diagnostics.to_result().unwrap();

        schema::build(Arc::new(schema), true)
    }

    #[test]
    fn recursive_composites() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
                "modelName": "List",
                "action": "createOne",
                "query": {
                  "selection": {
                    "$composites": true
                  }
                }
              }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::new(&recursive_composite_schema()).convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Err(
            Configuration(
                "$composites: true does not support recursive composite types.",
            ),
        )
        "###);
    }

    fn sibling_composite_schema() -> schema::QuerySchema {
        let schema_str = r#"
          generator client {
            provider        = "prisma-client-js"
          }
          
          datasource db {
            provider = "mongodb"
            url      = "mongodb://"
          }
          
          model User {
            id String @id @default(auto()) @map("_id") @db.ObjectId
          
            billingAddress Address
            shippingAddress Address
          }
          
          type Address {
            number Int
            street String
            zipCode Int
          }        
        "#;
        let mut schema = psl::validate(schema_str.into());

        schema.diagnostics.to_result().unwrap();

        schema::build(Arc::new(schema), true)
    }

    #[test]
    fn sibling_composites() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
                "modelName": "User",
                "action": "createOne",
                "query": {
                  "selection": {
                    "$composites": true
                  }
                }
              }"#,
        )
        .unwrap();

        let operation = JsonProtocolAdapter::new(&sibling_composite_schema()).convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Ok(
            Write(
                Selection {
                    name: "createOneUser",
                    alias: None,
                    arguments: [],
                    nested_selections: [
                        Selection {
                            name: "billingAddress",
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
                            name: "shippingAddress",
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
            ),
        )
        "###);
    }

    fn nested_sibling_composite_schema() -> schema::QuerySchema {
        let schema_str = r#"
          generator client {
            provider        = "prisma-client-js"
          }
          
          datasource db {
            provider = "mongodb"
            url      = "mongodb://"
          }
          
          model User {
            id         String    @id @default(auto()) @map("_id") @db.ObjectId
            billingAddress Address
            shippingAddress Address
          }
          
          type Address {
            streetAddress StreetAddress
            zipCode String
            city String
          }
          
          type StreetAddress {
            streetName String
            houseNumber String
          }       
        "#;
        let mut schema = psl::validate(schema_str.into());

        schema.diagnostics.to_result().unwrap();

        schema::build(Arc::new(schema), true)
    }

    #[test]
    pub fn nested_sibling_composites() {
        let query: JsonSingleQuery = serde_json::from_str(
            r#"{
                "modelName": "User",
                "action": "createOne",
                "query": {
                  "selection": {
                    "$composites": true
                  }
                }
              }"#,
        )
        .unwrap();

        let schema = nested_sibling_composite_schema();
        let mut adapter = JsonProtocolAdapter::new(&schema);
        let operation = adapter.convert_single(query);

        assert_debug_snapshot!(operation, @r###"
        Ok(
            Write(
                Selection {
                    name: "createOneUser",
                    alias: None,
                    arguments: [],
                    nested_selections: [
                        Selection {
                            name: "billingAddress",
                            alias: None,
                            arguments: [],
                            nested_selections: [
                                Selection {
                                    name: "streetAddress",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [
                                        Selection {
                                            name: "streetName",
                                            alias: None,
                                            arguments: [],
                                            nested_selections: [],
                                        },
                                        Selection {
                                            name: "houseNumber",
                                            alias: None,
                                            arguments: [],
                                            nested_selections: [],
                                        },
                                    ],
                                },
                                Selection {
                                    name: "zipCode",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                                Selection {
                                    name: "city",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                            ],
                        },
                        Selection {
                            name: "shippingAddress",
                            alias: None,
                            arguments: [],
                            nested_selections: [
                                Selection {
                                    name: "streetAddress",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [
                                        Selection {
                                            name: "streetName",
                                            alias: None,
                                            arguments: [],
                                            nested_selections: [],
                                        },
                                        Selection {
                                            name: "houseNumber",
                                            alias: None,
                                            arguments: [],
                                            nested_selections: [],
                                        },
                                    ],
                                },
                                Selection {
                                    name: "zipCode",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                                Selection {
                                    name: "city",
                                    alias: None,
                                    arguments: [],
                                    nested_selections: [],
                                },
                            ],
                        },
                    ],
                },
            ),
        )
        "###);
    }
}

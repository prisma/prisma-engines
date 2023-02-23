use crate::HandlerError;
use bigdecimal::{BigDecimal, FromPrimitive};
use graphql_parser::query::{
    Definition, Document, OperationDefinition, Selection as GqlSelection, SelectionSet, Value,
};
use query_core::query_document::*;

/// Protocol adapter for GraphQL -> Query Document.
///
/// GraphQL is mapped as following:
/// - Every field of a `query { ... }` or single selection block `{ ... }` is mapped to an `Operation::Read`.
/// - Every field of a single `mutation { ... }` is mapped to an `Operation::Write`.
/// - If the JSON payload specifies an operation name, only that specific operation is picked and the rest ignored.
/// - Fields on the queries are mapped to `Field`s, including arguments.
/// - Concrete values (e.g. in arguments) are mapped to `ArgumentValue`s.
///
/// Currently unsupported features:
/// - Fragments in any form.
/// - Variables.
/// - Subscription queries.
/// - Query names are ignored
pub struct GraphQLProtocolAdapter;

impl GraphQLProtocolAdapter {
    pub fn convert_query_to_operation(query: &str, operation_name: Option<String>) -> crate::Result<Operation> {
        let gql_doc = match graphql_parser::parse_query(query) {
            Ok(doc) => doc,
            Err(err)
                if err.to_string().contains("number too large to fit in target type")
                    | err.to_string().contains("number too small to fit in target type") =>
            {
                return Err(HandlerError::ValueFitError("Query parsing failure: A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers.".to_owned()));
            }
            err @ Err(_) => err?,
        };

        Self::convert(gql_doc, operation_name)
    }

    pub fn convert(gql_doc: Document<String>, operation: Option<String>) -> crate::Result<Operation> {
        let mut operations: Vec<Operation> = match operation {
            Some(ref op) => gql_doc
                .definitions
                .into_iter()
                .find(|def| Self::matches_operation(def, op))
                .ok_or_else(|| HandlerError::query_conversion(format!("Operation '{op}' does not match any query.")))
                .and_then(Self::convert_definition),

            None => gql_doc
                .definitions
                .into_iter()
                .map(Self::convert_definition)
                .collect::<crate::Result<Vec<Vec<Operation>>>>()
                .map(|r| r.into_iter().flatten().collect::<Vec<Operation>>()),
        }?;

        let operation = operations
            .pop()
            .ok_or_else(|| HandlerError::query_conversion("Document contained no operations."))?
            .dedup_selections();

        Ok(operation)
    }

    fn convert_definition(def: Definition<String>) -> crate::Result<Vec<Operation>> {
        match def {
            Definition::Fragment(f) => Err(HandlerError::unsupported_feature(
                "Fragment definition",
                format!("Fragment '{}', at position {}.", f.name, f.position),
            )),
            Definition::Operation(op) => match op {
                OperationDefinition::Subscription(s) => Err(HandlerError::unsupported_feature(
                    "Subscription query",
                    format!("At position {}.", s.position),
                )),
                OperationDefinition::SelectionSet(s) => Self::convert_query(s),
                OperationDefinition::Query(q) => Self::convert_query(q.selection_set),
                OperationDefinition::Mutation(m) => Self::convert_mutation(m.selection_set),
            },
        }
    }

    fn convert_query(selection_set: SelectionSet<String>) -> crate::Result<Vec<Operation>> {
        Self::convert_selection_set(selection_set).map(|fields| fields.into_iter().map(Operation::Read).collect())
    }

    fn convert_mutation(selection_set: SelectionSet<String>) -> crate::Result<Vec<Operation>> {
        Self::convert_selection_set(selection_set).map(|fields| fields.into_iter().map(Operation::Write).collect())
    }

    fn convert_selection_set(selection_set: SelectionSet<String>) -> crate::Result<Vec<Selection>> {
        selection_set
            .items
            .into_iter()
            .map(|item| match item {
                GqlSelection::Field(f) => {
                    let arguments: Vec<(String, ArgumentValue)> = f
                        .arguments
                        .into_iter()
                        .map(|(k, v)| Ok((k, Self::convert_value(v)?)))
                        .collect::<crate::Result<Vec<_>>>()?;

                    let nested_selections = Self::convert_selection_set(f.selection_set)?;

                    Ok(Selection::new(f.name, f.alias, arguments, nested_selections))
                }

                GqlSelection::FragmentSpread(fs) => Err(HandlerError::unsupported_feature(
                    "Fragment spread",
                    format!("Fragment '{}', at position {}.", fs.fragment_name, fs.position),
                )),

                GqlSelection::InlineFragment(i) => Err(HandlerError::unsupported_feature(
                    "Inline fragment",
                    format!("At position {}.", i.position),
                )),
            })
            .collect()
    }

    /// Checks if the given GraphQL definition matches the operation name that should be executed.
    fn matches_operation(def: &Definition<String>, operation: &str) -> bool {
        let check = |n: Option<&String>| n.filter(|name| name.as_str() == operation).is_some();

        match def {
            Definition::Fragment(_) => false,
            Definition::Operation(op) => match op {
                OperationDefinition::Subscription(s) => check(s.name.as_ref()),
                OperationDefinition::SelectionSet(_) => false,
                OperationDefinition::Query(q) => check(q.name.as_ref()),
                OperationDefinition::Mutation(m) => check(m.name.as_ref()),
            },
        }
    }

    fn convert_value(value: Value<String>) -> crate::Result<ArgumentValue> {
        match value {
            Value::Variable(name) => Err(HandlerError::unsupported_feature(
                "Variable usage",
                format!("Variable '{name}'."),
            )),
            Value::Int(i) => match i.as_i64() {
                Some(i) => Ok(ArgumentValue::int(i)),
                None => Err(HandlerError::query_conversion(format!("Invalid 64 bit integer: {i:?}"))),
            },
            Value::Float(f) => match BigDecimal::from_f64(f) {
                Some(dec) => Ok(ArgumentValue::float(dec)),
                None => Err(HandlerError::query_conversion(format!("invalid 64-bit float: {f:?}"))),
            },
            Value::String(s) => Ok(ArgumentValue::string(s)),
            Value::Boolean(b) => Ok(ArgumentValue::bool(b)),
            Value::Null => Ok(ArgumentValue::null()),
            Value::Enum(e) => Ok(ArgumentValue::r#enum(e)),
            Value::List(values) => {
                let values: Vec<ArgumentValue> = values
                    .into_iter()
                    .map(Self::convert_value)
                    .collect::<crate::Result<Vec<ArgumentValue>>>()?;

                Ok(ArgumentValue::list(values))
            }
            Value::Object(map) => {
                let values = map
                    .into_iter()
                    .map(|(k, v)| Self::convert_value(v).map(|v| (k, v)))
                    .collect::<crate::Result<ArgumentValueObject>>()?;

                Ok(ArgumentValue::object(values))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_single_query() {
        let query = r#"
            query findTheModelOperation {
                findOneModel(where: {a_number: {gte: 1}}) {
                    id,
                    large_number,
                    other {
                        name
                    }
                }
            }
        "#;

        let operation = GraphQLProtocolAdapter::convert_query_to_operation(query, None).unwrap();

        assert_eq!(operation.name(), "findOneModel");
        assert!(matches!(operation, Operation::Read(_)));

        let read = operation.into_read().unwrap();

        let where_args = ArgumentValue::object([(
            "a_number".to_string(),
            ArgumentValue::object([("gte".to_string(), ArgumentValue::int(1))]),
        )]);

        assert_eq!(read.arguments(), [("where".to_string(), where_args)]);

        let selections = Vec::from([
            Selection::new("id", None, [], Vec::new()),
            Selection::new("large_number", None, [], Vec::new()),
            Selection::new("other", None, [], Vec::from([Selection::new("name", None, [], [])])),
        ]);

        assert_eq!(read.nested_selections(), selections);
    }

    #[test]
    fn converts_single_mutation() {
        let query = r#"
        mutation {
                createOnePost(data: {
                    id: 1,
                    categories: {create: [{id: 1}, {id: 2}]}
            })  {
                id,
                categories {
                    id
                }
            }
        }
        "#;

        let operation = GraphQLProtocolAdapter::convert_query_to_operation(query, None).unwrap();

        assert_eq!(operation.name(), "createOnePost");
        assert!(matches!(operation, Operation::Write(_)));

        let write = operation.into_write().unwrap();

        let data_args = ArgumentValue::object([
            ("id".to_string(), ArgumentValue::int(1)),
            (
                "categories".to_string(),
                ArgumentValue::object([(
                    "create".to_string(),
                    ArgumentValue::list([
                        ArgumentValue::object([("id".to_string(), ArgumentValue::int(1))]),
                        ArgumentValue::object([("id".to_string(), ArgumentValue::int(2))]),
                    ]),
                )]),
            ),
        ]);
        println!("args {:?}", write.arguments());
        assert_eq!(write.arguments(), [("data".to_string(), data_args)]);
    }
}

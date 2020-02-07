use crate::{error::PrismaError, PrismaResult};
use graphql_parser::query::{
    Definition, Document, OperationDefinition, Selection as GqlSelection, SelectionSet, Value,
};
use query_core::query_document::*;
use rust_decimal::{prelude::FromPrimitive, Decimal};
use std::collections::BTreeMap;

/// Protocol adapter for GraphQL -> Query Document.
///
/// GraphQL is mapped as following:
/// - Every field of a `query { ... }` or single selection block `{ ... }` is mapped to an `Operation::Read`.
/// - Every field of a single `mutation { ... }` is mapped to an `Operation::Write`.
/// - If the JSON payload specifies an operation name, only that specific operation is picked and the rest ignored.
/// - Fields on the queries are mapped to `Field`s, including arguments.
/// - Concrete values (e.g. in arguments) are mapped to `QueryValue`s.
///
/// Currently unsupported features:
/// - Fragments in any form.
/// - Variables.
/// - Subscription queries.
/// - Query names are ignored
pub struct GraphQLProtocolAdapter;

impl GraphQLProtocolAdapter {
    pub fn convert(gql_doc: Document, operation: Option<String>) -> PrismaResult<Operation> {
        let mut operations: Vec<Operation> = match operation {
            Some(ref op) => gql_doc
                .definitions
                .into_iter()
                .find(|def| Self::matches_operation(def, op))
                .ok_or_else(|| {
                    PrismaError::QueryConversionError(format!("Operation '{}' does not match any query.", op))
                })
                .and_then(Self::convert_definition),

            None => gql_doc
                .definitions
                .into_iter()
                .map(Self::convert_definition)
                .collect::<PrismaResult<Vec<Vec<Operation>>>>()
                .map(|r| r.into_iter().flatten().collect::<Vec<Operation>>()),
        }?;

        let operation = operations
            .pop()
            .ok_or_else(|| PrismaError::QueryConversionError("Document contained no operations.".into()))?
            .dedup_selections();

        Ok(operation)
    }

    fn convert_definition(def: Definition) -> PrismaResult<Vec<Operation>> {
        match def {
            Definition::Fragment(f) => Err(PrismaError::UnsupportedFeatureError(
                "Fragment definition",
                format!("Fragment '{}', at position {}.", f.name, f.position),
            )),
            Definition::Operation(op) => match op {
                OperationDefinition::Subscription(s) => Err(PrismaError::UnsupportedFeatureError(
                    "Subscription query",
                    format!("At position {}.", s.position),
                )),
                OperationDefinition::SelectionSet(s) => Self::convert_query(s),
                OperationDefinition::Query(q) => Self::convert_query(q.selection_set),
                OperationDefinition::Mutation(m) => Self::convert_mutation(m.selection_set),
            },
        }
    }

    fn convert_query(selection_set: SelectionSet) -> PrismaResult<Vec<Operation>> {
        Self::convert_selection_set(selection_set)
            .map(|fields| fields.into_iter().map(|field| Operation::Read(field)).collect())
    }

    fn convert_mutation(selection_set: SelectionSet) -> PrismaResult<Vec<Operation>> {
        Self::convert_selection_set(selection_set).map(|fields| {
            fields
                .into_iter()
                .map(|selection| Operation::Write(selection))
                .collect()
        })
    }

    fn convert_selection_set(selection_set: SelectionSet) -> PrismaResult<Vec<Selection>> {
        selection_set
            .items
            .into_iter()
            .map(|item| match item {
                GqlSelection::Field(f) => {
                    let arguments: Vec<(String, QueryValue)> = f
                        .arguments
                        .into_iter()
                        .map(|(k, v)| Ok((k, Self::convert_value(v)?)))
                        .collect::<PrismaResult<Vec<_>>>()?;

                    let mut builder = Selection::builder(f.name);
                    builder.set_arguments(arguments);
                    builder.nested_selections(Self::convert_selection_set(f.selection_set)?);

                    if let Some(alias) = f.alias {
                        builder.alias(alias);
                    };

                    Ok(builder.build())
                }

                GqlSelection::FragmentSpread(fs) => Err(PrismaError::UnsupportedFeatureError(
                    "Fragment spread",
                    format!("Fragment '{}', at position {}.", fs.fragment_name, fs.position),
                )),

                GqlSelection::InlineFragment(i) => Err(PrismaError::UnsupportedFeatureError(
                    "Inline fragment",
                    format!("At position {}.", i.position),
                )),
            })
            .collect()
    }

    /// Checks if the given GraphQL definition matches the operation name that should be executed.
    fn matches_operation(def: &Definition, operation: &str) -> bool {
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

    fn convert_value(value: Value) -> PrismaResult<QueryValue> {
        match value {
            Value::Variable(name) => Err(PrismaError::UnsupportedFeatureError(
                "Variable usage",
                format!("Variable '{}'.", name),
            )),
            Value::Int(i) => match i.as_i64() {
                Some(i) => Ok(QueryValue::Int(i)),
                None => Err(PrismaError::QueryConversionError(format!(
                    "Invalid 64 bit integer: {:?}",
                    i
                ))),
            },
            Value::Float(f) => match Decimal::from_f64(f) {
                Some(dec) => Ok(QueryValue::Float(dec)),
                None => Err(PrismaError::QueryConversionError(format!(
                    "invalid 64-bit float: {:?}",
                    f
                ))),
            },
            Value::String(s) => Ok(QueryValue::String(s)),
            Value::Boolean(b) => Ok(QueryValue::Boolean(b)),
            Value::Null => Ok(QueryValue::Null),
            Value::Enum(e) => Ok(QueryValue::Enum(e)),
            Value::List(values) => {
                let values: Vec<QueryValue> = values
                    .into_iter()
                    .map(Self::convert_value)
                    .collect::<PrismaResult<Vec<QueryValue>>>()?;

                Ok(QueryValue::List(values))
            }
            Value::Object(map) => {
                let values = map
                    .into_iter()
                    .map(|(k, v)| Self::convert_value(v).map(|v| (k, v)))
                    .collect::<PrismaResult<BTreeMap<String, QueryValue>>>()?;

                Ok(QueryValue::Object(values))
            }
        }
    }
}

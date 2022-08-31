use super::GraphQLProtocolAdapter;
use crate::HandlerError;
use graphql_parser as gql;
use query_core::{BatchDocument, Operation, QueryDocument};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum GraphQlBody {
    Single(SingleQuery),
    Multi(MultiQuery),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SingleQuery {
    query: String,
    operation_name: Option<String>,
    variables: HashMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiQuery {
    batch: Vec<SingleQuery>,
    transaction: bool,
}

impl MultiQuery {
    pub fn new(batch: Vec<SingleQuery>, transaction: bool) -> Self {
        Self { batch, transaction }
    }
}

impl From<String> for SingleQuery {
    fn from(query: String) -> Self {
        SingleQuery {
            query,
            operation_name: None,
            variables: HashMap::new(),
        }
    }
}

impl From<&str> for SingleQuery {
    fn from(query: &str) -> Self {
        String::from(query).into()
    }
}

impl ToString for GraphQlBody {
    fn to_string(&self) -> String {
        match self {
            GraphQlBody::Single(single) => single.query.clone(),
            _ => String::default(),
        }
    }
}

impl GraphQlBody {
    /// Convert a `GraphQlBody` into a `QueryDocument`.
    pub(crate) fn into_doc(self) -> crate::Result<QueryDocument> {
        match self {
            GraphQlBody::Single(body) => {
                let gql_doc = match gql::parse_query(&body.query) {
                    Ok(doc) => doc,
                    Err(err)
                        if err.to_string().contains("number too large to fit in target type")
                            | err.to_string().contains("number too small to fit in target type") =>
                    {
                        return Err(HandlerError::ValueFitError("Query parsing failure: A number used in the query does not fit into a 64 bit signed integer. Consider using `BigInt` as field type if you're trying to store large integers.".to_owned()));
                    }
                    err @ Err(_) => err?,
                };
                let operation = GraphQLProtocolAdapter::convert(gql_doc, body.operation_name)?;

                Ok(QueryDocument::Single(operation))
            }
            GraphQlBody::Multi(bodies) => {
                let operations: crate::Result<Vec<Operation>> = bodies
                    .batch
                    .into_iter()
                    .map(|body| {
                        let gql_doc = gql::parse_query(&body.query)?;
                        GraphQLProtocolAdapter::convert(gql_doc, body.operation_name)
                    })
                    .collect();

                Ok(QueryDocument::Multi(BatchDocument::new(
                    operations?,
                    bodies.transaction,
                )))
            }
        }
    }
}

use super::GraphQLProtocolAdapter;
use query_core::{BatchDocument, BatchDocumentTransaction, Operation, QueryDocument};
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
    isolation_level: Option<String>,
}

impl MultiQuery {
    pub fn new(batch: Vec<SingleQuery>, transaction: bool, isolation_level: Option<String>) -> Self {
        Self {
            batch,
            transaction,
            isolation_level,
        }
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

impl GraphQlBody {
    /// Convert a `GraphQlBody` into a `QueryDocument`.
    pub fn into_doc(self) -> crate::Result<QueryDocument> {
        match self {
            GraphQlBody::Single(body) => {
                let operation = GraphQLProtocolAdapter::convert_query_to_operation(&body.query, body.operation_name)?;

                Ok(QueryDocument::Single(operation))
            }
            GraphQlBody::Multi(bodies) => {
                let operations: crate::Result<Vec<Operation>> = bodies
                    .batch
                    .into_iter()
                    .map(|body| GraphQLProtocolAdapter::convert_query_to_operation(&body.query, body.operation_name))
                    .collect();
                let transaction = if bodies.transaction {
                    Some(BatchDocumentTransaction::new(bodies.isolation_level))
                } else {
                    None
                };

                Ok(QueryDocument::Multi(BatchDocument::new(operations?, transaction)))
            }
        }
    }
}

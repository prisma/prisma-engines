use super::protocol_adapter::GraphQLProtocolAdapter;
use crate::{context::PrismaContext, PrismaResponse, PrismaResult};

use futures::{future, FutureExt};
use graphql_parser as gql;
use indexmap::IndexMap;
use query_core::{
    response_ir, BatchDocument, CompactedDocument, CoreError, Item, Operation, QueryDocument, QueryValue, Responses,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, panic::AssertUnwindSafe, sync::Arc};

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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum GraphQlBody {
    Single(SingleQuery),
    Multi(MultiQuery),
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
    pub(crate) fn into_doc(self) -> PrismaResult<QueryDocument> {
        match self {
            GraphQlBody::Single(body) => {
                let gql_doc = gql::parse_query(&body.query)?;
                let operation = GraphQLProtocolAdapter::convert(gql_doc, body.operation_name)?;

                Ok(QueryDocument::Single(operation))
            }
            GraphQlBody::Multi(bodies) => {
                let operations: PrismaResult<Vec<Operation>> = bodies
                    .batch
                    .into_iter()
                    .map(|body| {
                        let gql_doc = gql::parse_query(&body.query)?;
                        GraphQLProtocolAdapter::convert(gql_doc, body.operation_name)
                    })
                    .collect();

                Ok(QueryDocument::Multi(BatchDocument::new(operations?)))
            }
        }
    }
}

/// Handle a Graphql request.
pub(crate) async fn handle(body: GraphQlBody, cx: Arc<PrismaContext>) -> PrismaResponse {
    debug!("Incoming GraphQL query: {:?}", body);

    match body.into_doc() {
        Ok(QueryDocument::Single(query)) => handle_single_query(query, cx.clone()).await,
        Ok(QueryDocument::Multi(batch)) => match batch.compact() {
            BatchDocument::Multi(batch) => handle_batch(batch, &cx).await,
            BatchDocument::Compact(compacted) => handle_compacted(compacted, &cx).await,
        },
        Err(err) => {
            let mut responses = response_ir::Responses::default();
            responses.insert_error(err);

            PrismaResponse::Single(responses)
        }
    }
}

async fn handle_single_query(query: Operation, ctx: Arc<PrismaContext>) -> PrismaResponse {
    use user_facing_errors::Error;

    let responses = match AssertUnwindSafe(handle_graphql_query(query, &*ctx))
        .catch_unwind()
        .await
    {
        Ok(Ok(responses)) => responses,
        Ok(Err(err)) => {
            let mut responses = response_ir::Responses::default();
            responses.insert_error(err);
            responses
        }
        // panicked
        Err(err) => {
            let mut responses = response_ir::Responses::default();
            let error = Error::from_panic_payload(&err);

            responses.insert_error(error);
            responses
        }
    };

    PrismaResponse::Single(responses)
}

async fn handle_batch(queries: Vec<Operation>, ctx: &Arc<PrismaContext>) -> PrismaResponse {
    let mut futures = Vec::with_capacity(queries.len());

    for operation in queries.into_iter() {
        futures.push(tokio::spawn(handle_single_query(operation, ctx.clone())));
    }

    let responses = future::join_all(futures)
        .await
        .into_iter()
        .map(|res| res.expect("IO Error in tokio::spawn"))
        .collect();

    PrismaResponse::Multi(responses)
}

async fn handle_compacted(document: CompactedDocument, ctx: &Arc<PrismaContext>) -> PrismaResponse {
    use user_facing_errors::Error;

    let plural_name = document.plural_name();
    let singular_name = document.single_name();
    let keys = document.keys;
    let arguments = document.arguments;
    let nested_selection = document.nested_selection;

    match AssertUnwindSafe(handle_graphql_query(document.operation, ctx))
        .catch_unwind()
        .await
    {
        Ok(Ok(mut responses)) => {
            // We find the response data and make a hash from the given unique
            // keys.
            let data = responses
                .take_data(plural_name)
                .unwrap()
                .into_list()
                .unwrap()
                .index_by(keys.as_slice());

            let results = arguments
                .into_iter()
                .map(|args| {
                    let vals: Vec<QueryValue> = args.into_iter().map(|(_, v)| v).collect();
                    let mut responses = Responses::with_capacity(1);

                    // Copying here is mandatory due to some of the queries
                    // might be repeated with the same arguments in the original
                    // batch. We need to give the same answer for both of them.
                    match data.get(&vals) {
                        Some(result) => {
                            // Filter out all the keys not selected in the
                            // original query.
                            let result: IndexMap<String, Item> = result
                                .clone()
                                .into_iter()
                                .filter(|(k, _)| nested_selection.contains(k))
                                .collect();

                            responses.insert_data(&singular_name, Item::Map(result));
                        }
                        _ => {
                            responses.insert_data(&singular_name, Item::null());
                        }
                    }

                    PrismaResponse::Single(responses)
                })
                .collect();

            PrismaResponse::Multi(results)
        }
        Ok(Err(err)) => {
            let mut responses = response_ir::Responses::default();
            responses.insert_error(err);
            PrismaResponse::Single(responses)
        }
        // panicked
        Err(err) => {
            let mut responses = response_ir::Responses::default();
            let error = Error::from_panic_payload(&err);

            responses.insert_error(error);
            PrismaResponse::Single(responses)
        }
    }
}

async fn handle_graphql_query(query_doc: Operation, ctx: &PrismaContext) -> PrismaResult<response_ir::Responses> {
    ctx.executor
        .execute(query_doc, Arc::clone(ctx.query_schema()))
        .await
        .map_err(|err| {
            debug!("{}", err);
            let ce: CoreError = err.into();
            ce.into()
        })
}

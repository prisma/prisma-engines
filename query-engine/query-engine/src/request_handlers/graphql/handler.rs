use super::{protocol_adapter::GraphQLProtocolAdapter, GQLResponse};
use crate::{context::PrismaContext, PrismaError, PrismaRequest, PrismaResponse, PrismaResult, RequestHandler};
use async_trait::async_trait;
use futures::FutureExt;
use graphql_parser as gql;
use indexmap::IndexMap;
use query_core::{BatchDocument, CompactedDocument, Item, Operation, QueryDocument, QueryValue, ResponseData};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, convert::TryFrom, panic::AssertUnwindSafe, sync::Arc};

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

impl TryFrom<GraphQlBody> for QueryDocument {
    type Error = PrismaError;

    fn try_from(body: GraphQlBody) -> PrismaResult<Self> {
        match body {
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

                Ok(QueryDocument::Multi(BatchDocument::new(
                    operations?,
                    bodies.transaction,
                )))
            }
        }
    }
}

pub struct GraphQlRequestHandler;

#[allow(unused_variables)]
#[async_trait]
impl RequestHandler for GraphQlRequestHandler {
    type Body = GraphQlBody;

    async fn handle<S>(&self, req: S, ctx: &Arc<PrismaContext>) -> PrismaResponse
    where
        S: Into<PrismaRequest<Self::Body>> + Send + Sync + 'static,
    {
        let request = req.into();
        debug!("Incoming GraphQL query: {:?}", request.body);

        match QueryDocument::try_from(request.body) {
            Ok(QueryDocument::Single(query)) => handle_single_query(query, ctx.clone()).await,
            Ok(QueryDocument::Multi(batch)) => match batch.compact() {
                BatchDocument::Multi(batch, transactional) => handle_batch(batch, transactional, ctx).await,
                BatchDocument::Compact(compacted) => handle_compacted(compacted, ctx).await,
            },
            Err(err) => PrismaResponse::Single(err.into()),
        }
    }
}

async fn handle_single_query(query: Operation, ctx: Arc<PrismaContext>) -> PrismaResponse {
    use user_facing_errors::Error;

    let gql_response = match AssertUnwindSafe(handle_graphql_query(query, &*ctx))
        .catch_unwind()
        .await
    {
        Ok(Ok(responses)) => responses.into(),
        Ok(Err(err)) => err.into(),
        Err(err) => {
            // panicked
            let error = Error::from_panic_payload(&err);
            error.into()
        }
    };

    PrismaResponse::Single(gql_response)
}

async fn handle_batch(queries: Vec<Operation>, transactional: bool, ctx: &Arc<PrismaContext>) -> PrismaResponse {
    use user_facing_errors::Error;

    match AssertUnwindSafe(
        ctx.executor
            .execute_batch(queries, transactional, ctx.query_schema().clone()),
    )
    .catch_unwind()
    .await
    {
        Ok(Ok(responses)) => {
            let gql_responses = responses
                .into_iter()
                .map(|response| match response {
                    Ok(data) => PrismaResponse::Single(data.into()),
                    Err(err) => PrismaResponse::Single(err.into()),
                })
                .collect();

            PrismaResponse::Multi(gql_responses)
        }
        Ok(Err(err)) => PrismaResponse::Single(err.into()),
        Err(err) => {
            // panicked
            let error = Error::from_panic_payload(&err);
            let resp: GQLResponse = error.into();

            PrismaResponse::Single(resp)
        }
    }
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
        Ok(Ok(response_data)) => {
            let mut gql_response: GQLResponse = response_data.into();

            // We find the response data and make a hash from the given unique keys.
            let data = gql_response
                .take_data(plural_name)
                .unwrap()
                .into_list()
                .unwrap()
                .index_by(keys.as_slice());

            let results = arguments
                .into_iter()
                .map(|args| {
                    let vals: Vec<QueryValue> = args.into_iter().map(|(_, v)| v).collect();
                    let mut responses = GQLResponse::with_capacity(1);

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

        Ok(Err(err)) => PrismaResponse::Single(err.into()),

        // panicked
        Err(err) => {
            let error = Error::from_panic_payload(&err);
            PrismaResponse::Single(error.into())
        }
    }
}

async fn handle_graphql_query(query_doc: Operation, ctx: &PrismaContext) -> PrismaResult<ResponseData> {
    Ok(ctx.executor.execute(query_doc, Arc::clone(ctx.query_schema())).await?)
}

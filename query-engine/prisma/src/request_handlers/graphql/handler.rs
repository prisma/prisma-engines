use super::protocol_adapter::GraphQLProtocolAdapter;
use crate::{context::PrismaContext, PrismaRequest, PrismaResponse, PrismaResult, RequestHandler};
use async_trait::async_trait;
use futures::{future, FutureExt};
use graphql_parser as gql;
use query_core::{response_ir, CoreError};
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

        match request.body {
            GraphQlBody::Single(query) => handle_single_query(query, ctx.clone()).await,
            GraphQlBody::Multi(queries) => {
                let mut futures = Vec::with_capacity(queries.batch.len());

                for query in queries.batch.into_iter() {
                    futures.push(tokio::spawn(handle_single_query(query, ctx.clone())));
                }

                let responses = future::join_all(futures)
                    .await
                    .into_iter()
                    .map(|res| res.expect("IO Error in tokio::spawn"))
                    .collect();

                PrismaResponse::Multi(responses)
            }
        }
    }
}

async fn handle_single_query(query: SingleQuery, ctx: Arc<PrismaContext>) -> PrismaResponse {
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

async fn handle_graphql_query(body: SingleQuery, ctx: &PrismaContext) -> PrismaResult<response_ir::Responses> {
    debug!("Incoming GQL query: {:?}", &body.query);
    debug!("Operation: {:?}", body.operation_name);

    let gql_doc = gql::parse_query(&body.query)?;
    let query_doc = GraphQLProtocolAdapter::convert(gql_doc, body.operation_name)?;

    ctx.executor
        .execute(query_doc, Arc::clone(ctx.query_schema()))
        .await
        .map_err(|err| {
            debug!("{}", err);
            let ce: CoreError = err.into();
            ce.into()
        })
}

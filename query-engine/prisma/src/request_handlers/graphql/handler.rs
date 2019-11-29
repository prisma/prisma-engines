use super::protocol_adapter::GraphQLProtocolAdapter;
use crate::{context::PrismaContext, serializers::json, PrismaRequest, PrismaResult, RequestHandler};
use async_trait::async_trait;
use graphql_parser as gql;
use query_core::{response_ir, CoreError};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQlBody {
    query: String,
    operation_name: Option<String>,
    variables: HashMap<String, String>,
}

pub struct GraphQlRequestHandler;

#[allow(unused_variables)]
#[async_trait]
impl RequestHandler for GraphQlRequestHandler {
    type Body = GraphQlBody;

    async fn handle<S>(&self, req: S, ctx: &PrismaContext) -> serde_json::Value
    where
        S: Into<PrismaRequest<Self::Body>> + Send + Sync + 'static,
    {
        let responses = match handle_graphql_query(req.into(), ctx).await {
            Ok(responses) => responses,
            Err(err) => vec![err.into()],
        };

        json::serialize(responses)
    }
}

async fn handle_graphql_query(
    req: PrismaRequest<GraphQlBody>,
    ctx: &PrismaContext,
) -> PrismaResult<Vec<response_ir::Response>> {
    debug!("Incoming GQL query: {:?}", &req.body.query);

    let gql_doc = gql::parse_query(&req.body.query)?;
    let query_doc = GraphQLProtocolAdapter::convert(gql_doc, req.body.operation_name)?;

    ctx.executor
        .execute(query_doc, Arc::clone(ctx.query_schema()))
        .await
        .map_err(|err| {
            debug!("{}", err);
            let ce: CoreError = err.into();
            ce.into()
        })
}

use super::{GQLBatchResponse, GQLResponse, GraphQlBody};
use crate::PrismaResponse;
use futures::FutureExt;
use indexmap::IndexMap;
use query_core::{
    BatchDocument, CompactedDocument, Item, Operation, QueryDocument, QueryExecutor, QuerySchemaRef, QueryValue,
    ResponseData,
};
use std::panic::AssertUnwindSafe;

pub struct GraphQlHandler<'a> {
    executor: &'a (dyn QueryExecutor + Send + Sync + 'a),
    query_schema: &'a QuerySchemaRef,
}

impl<'a> GraphQlHandler<'a> {
    pub fn new(executor: &'a (dyn QueryExecutor + Send + Sync + 'a), query_schema: &'a QuerySchemaRef) -> Self {
        Self { executor, query_schema }
    }

    pub async fn handle(&self, body: GraphQlBody) -> PrismaResponse {
        tracing::debug!("Incoming GraphQL query: {:?}", body);

        match body.into_doc() {
            Ok(QueryDocument::Single(query)) => self.handle_single(query).await,
            Ok(QueryDocument::Multi(batch)) => match batch.compact() {
                BatchDocument::Multi(batch, transactional) => self.handle_batch(batch, transactional).await,
                BatchDocument::Compact(compacted) => self.handle_compacted(compacted).await,
            },
            Err(err) => PrismaResponse::Single(err.into()),
        }
    }

    async fn handle_single(&self, query: Operation) -> PrismaResponse {
        use user_facing_errors::Error;

        let gql_response = match AssertUnwindSafe(self.handle_graphql(query)).catch_unwind().await {
            Ok(Ok(response)) => response.into(),
            Ok(Err(err)) => err.into(),
            Err(err) => {
                // panicked
                let error = Error::from_panic_payload(&err);
                error.into()
            }
        };

        PrismaResponse::Single(gql_response)
    }

    async fn handle_batch(&self, queries: Vec<Operation>, transactional: bool) -> PrismaResponse {
        use user_facing_errors::Error;

        match AssertUnwindSafe(
            self.executor
                .execute_batch(queries, transactional, self.query_schema.clone()),
        )
        .catch_unwind()
        .await
        {
            Ok(Ok(responses)) => {
                let gql_responses: Vec<GQLResponse> = responses
                    .into_iter()
                    .map(|response| match response {
                        Ok(data) => data.into(),
                        Err(err) => err.into(),
                    })
                    .collect();

                PrismaResponse::Multi(gql_responses.into())
            }
            Ok(Err(err)) => PrismaResponse::Multi(err.into()),
            Err(err) => {
                // panicked
                let error = Error::from_panic_payload(&err);
                let resp: GQLBatchResponse = error.into();

                PrismaResponse::Multi(resp)
            }
        }
    }

    async fn handle_compacted(&self, document: CompactedDocument) -> PrismaResponse {
        use user_facing_errors::Error;

        let plural_name = document.plural_name();
        let singular_name = document.single_name();
        let keys = document.keys;
        let arguments = document.arguments;
        let nested_selection = document.nested_selection;

        match AssertUnwindSafe(self.handle_graphql(document.operation))
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

                let results: Vec<GQLResponse> = arguments
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

                        responses
                    })
                    .collect();

                PrismaResponse::Multi(results.into())
            }

            Ok(Err(err)) => PrismaResponse::Multi(err.into()),

            // panicked
            Err(err) => {
                let error = Error::from_panic_payload(&err);
                PrismaResponse::Multi(error.into())
            }
        }
    }

    async fn handle_graphql(&self, query_doc: Operation) -> query_core::Result<ResponseData> {
        self.executor.execute(query_doc, self.query_schema.clone()).await
    }
}

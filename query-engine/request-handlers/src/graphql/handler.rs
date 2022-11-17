use super::{GQLBatchResponse, GQLResponse, GraphQlBody};
use crate::PrismaResponse;
use futures::FutureExt;
use indexmap::IndexMap;
use query_core::{
    schema::QuerySchemaRef, BatchDocument, BatchDocumentTransaction, CompactedDocument, Item, Operation, QueryDocument,
    QueryExecutor, ResponseData, TxId,
};
use std::{fmt, panic::AssertUnwindSafe};

pub struct GraphQlHandler<'a> {
    executor: &'a (dyn QueryExecutor + Send + Sync + 'a),
    query_schema: &'a QuerySchemaRef,
}

impl<'a> fmt::Debug for GraphQlHandler<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GraphQlHandler").finish()
    }
}

impl<'a> GraphQlHandler<'a> {
    pub fn new(executor: &'a (dyn QueryExecutor + Send + Sync + 'a), query_schema: &'a QuerySchemaRef) -> Self {
        Self { executor, query_schema }
    }

    pub async fn handle(&self, body: GraphQlBody, tx_id: Option<TxId>, trace_id: Option<String>) -> PrismaResponse {
        tracing::debug!("Incoming GraphQL query: {:?}", &body);

        match body.into_doc() {
            Ok(QueryDocument::Single(query)) => self.handle_single(query, tx_id, trace_id).await,
            Ok(QueryDocument::Multi(batch)) => match batch.compact(self.query_schema) {
                BatchDocument::Multi(batch, transaction) => {
                    self.handle_batch(batch, transaction, tx_id, trace_id).await
                }
                BatchDocument::Compact(compacted) => self.handle_compacted(compacted, tx_id, trace_id).await,
            },
            Err(err) => match err.as_known_error() {
                Some(transformed) => PrismaResponse::Single(user_facing_errors::Error::new_known(transformed).into()),
                None => PrismaResponse::Single(err.into()),
            },
        }
    }

    async fn handle_single(&self, query: Operation, tx_id: Option<TxId>, trace_id: Option<String>) -> PrismaResponse {
        use user_facing_errors::Error;

        let gql_response = match AssertUnwindSafe(self.handle_graphql(query, tx_id, trace_id))
            .catch_unwind()
            .await
        {
            Ok(Ok(response)) => response.into(),
            Ok(Err(err)) => err.into(),
            Err(err) => {
                // panicked
                let error = Error::from_panic_payload(err);
                error.into()
            }
        };

        PrismaResponse::Single(gql_response)
    }

    async fn handle_batch(
        &self,
        queries: Vec<Operation>,
        transaction: Option<BatchDocumentTransaction>,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> PrismaResponse {
        use user_facing_errors::Error;

        match AssertUnwindSafe(self.executor.execute_all(
            tx_id,
            queries,
            transaction,
            self.query_schema.clone(),
            trace_id,
        ))
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
                let error = Error::from_panic_payload(err);
                let resp: GQLBatchResponse = error.into();

                PrismaResponse::Multi(resp)
            }
        }
    }

    async fn handle_compacted(
        &self,
        document: CompactedDocument,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> PrismaResponse {
        use user_facing_errors::Error;

        let plural_name = document.plural_name();
        let singular_name = document.single_name();
        let keys = document.keys;
        let arguments = document.arguments;
        let nested_selection = document.nested_selection;

        match AssertUnwindSafe(self.handle_graphql(document.operation, tx_id, trace_id))
            .catch_unwind()
            .await
        {
            Ok(Ok(response_data)) => {
                let mut gql_response: GQLResponse = response_data.into();

                // At this point, many findUnique queries were converted to a single findMany query and that query was run.
                // This means we have a list of results and we need to map each result back to their original findUnique query.
                // `data` is the piece of logic that allows us to do that mapping.
                // It takes the findMany response and converts it to a map of arguments to result.
                // Let's take an example. Given the following batched queries:
                // [
                //    findUnique(where: { id: 1, name: "Bob" }) { id name age },
                //    findUnique(where: { id: 2, name: "Alice" }) { id name age }
                // ]
                // 1. This gets converted to: findMany(where: { OR: [{ id: 1, name: "Bob" }, { id: 2, name: "Alice" }] }) { id name age }
                // 2. Say we get the following result back: [{ id: 1, name: "Bob", age: 18 }, { id: 2, name: "Alice", age: 27 }]
                // 3. We know the inputted arguments are ["id", "name"]
                // 4. So we go over the result and build the following list:
                // [
                //  ({ id: 1, name: "Bob" },   { id: 1, name: "Bob", age: 18 }),
                //  ({ id: 2, name: "Alice" }, { id: 2, name: "Alice", age: 27 })
                // ]
                // 5. Now, given the original findUnique queries and that list, we can easily find back which arguments maps to which result
                // [
                //    findUnique(where: { id: 1, name: "Bob" }) { id name age } -> { id: 1, name: "Bob", age: 18 }
                //    findUnique(where: { id: 2, name: "Alice" }) { id name age } -> { id: 2, name: "Alice", age: 27 }
                // ]
                let args_to_results = gql_response
                    .take_data(plural_name)
                    .unwrap()
                    .into_list()
                    .unwrap()
                    .index_by(keys.as_slice());

                let results: Vec<GQLResponse> = arguments
                    .into_iter()
                    .map(|args| {
                        let mut responses = GQLResponse::with_capacity(1);

                        // This is step 5 of the comment above.
                        // Copying here is mandatory due to some of the queries
                        // might be repeated with the same arguments in the original
                        // batch. We need to give the same answer for both of them.
                        match args_to_results.iter().find(|(a, _)| *a == args) {
                            Some((_, result)) => {
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
                let error = Error::from_panic_payload(err);
                PrismaResponse::Multi(error.into())
            }
        }
    }

    async fn handle_graphql(
        &self,
        query_doc: Operation,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> query_core::Result<ResponseData> {
        self.executor
            .execute(tx_id, query_doc, self.query_schema.clone(), trace_id)
            .await
    }
}

use super::{GQLBatchResponse, GQLResponse};
use crate::{PrismaResponse, RequestBody};
use futures::FutureExt;
use indexmap::IndexMap;
use prisma_models::{parse_datetime, stringify_datetime, PrismaValue};
use query_core::{
    protocol::EngineProtocol,
    response_ir::{Item, ResponseData},
    schema::QuerySchemaRef,
    BatchDocument, BatchDocumentTransaction, CompactedDocument, Operation, QueryDocument, QueryExecutor, TxId,
};
use std::{collections::HashMap, fmt, panic::AssertUnwindSafe};

type ArgsToResult = (HashMap<String, PrismaValue>, IndexMap<String, Item>);

pub struct RequestHandler<'a> {
    executor: &'a (dyn QueryExecutor + Send + Sync + 'a),
    query_schema: &'a QuerySchemaRef,
    engine_protocol: EngineProtocol,
}

impl<'a> fmt::Debug for RequestHandler<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequestHandler").finish()
    }
}

impl<'a> RequestHandler<'a> {
    pub fn new(
        executor: &'a (dyn QueryExecutor + Send + Sync + 'a),
        query_schema: &'a QuerySchemaRef,
        engine_protocol: EngineProtocol,
    ) -> Self {
        Self {
            executor,
            query_schema,
            engine_protocol,
        }
    }

    pub async fn handle(&self, body: RequestBody, tx_id: Option<TxId>, trace_id: Option<String>) -> PrismaResponse {
        tracing::debug!("Incoming GraphQL query: {:?}", &body);

        match body.into_doc(self.query_schema) {
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

        let gql_response = match AssertUnwindSafe(self.handle_request(query, tx_id, trace_id))
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
            self.engine_protocol,
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

        match AssertUnwindSafe(self.handle_request(document.operation, tx_id, trace_id))
            .catch_unwind()
            .await
        {
            Ok(Ok(response_data)) => {
                let mut gql_response: GQLResponse = response_data.into();

                // At this point, many findUnique queries were converted to a single findMany query and that query was run.
                // This means we have a list of results and we need to map each result back to their original findUnique query.
                // `args_to_results` is the data-structure that allows us to do that mapping.
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
                let args_to_results: Vec<ArgsToResult> = gql_response
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
                        match Self::find_original_result_from_args(&args_to_results, &args) {
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
                let error = Error::from_panic_payload(err);
                PrismaResponse::Multi(error.into())
            }
        }
    }

    async fn handle_request(
        &self,
        query_doc: Operation,
        tx_id: Option<TxId>,
        trace_id: Option<String>,
    ) -> query_core::Result<ResponseData> {
        self.executor
            .execute(
                tx_id,
                query_doc,
                self.query_schema.clone(),
                trace_id,
                self.engine_protocol,
            )
            .await
    }

    fn find_original_result_from_args<'b>(
        args_to_results: &'b [ArgsToResult],
        input_args: &'b HashMap<String, PrismaValue>,
    ) -> Option<&'b IndexMap<String, Item>> {
        args_to_results
            .iter()
            .find(|(arg_from_result, _)| Self::compare_args(arg_from_result, input_args))
            .map(|(_, result)| result)
    }

    fn compare_args(left: &HashMap<String, PrismaValue>, right: &HashMap<String, PrismaValue>) -> bool {
        left.iter().all(|(key, left_value)| {
            right
                .get(key)
                .map_or(false, |right_value| Self::compare_values(left_value, right_value))
        })
    }

    /// Compares two PrismaValues but treats DateTime and String as equal when their parsed/stringified versions are equal.
    /// We need this when comparing user-inputted values with query response values in the context of compacted queries.
    /// User-inputted datetimes are coerced as `PrismaValue::DateTime` but response (and thus serialized) datetimes are `PrismaValue::String`.
    /// This should likely _not_ be used outside of this specific context.
    fn compare_values(left: &PrismaValue, right: &PrismaValue) -> bool {
        match (left, right) {
            (PrismaValue::String(t1), PrismaValue::DateTime(t2))
            | (PrismaValue::DateTime(t2), PrismaValue::String(t1)) => parse_datetime(t1)
                .map(|t1| &t1 == t2)
                .unwrap_or_else(|_| t1 == stringify_datetime(t2).as_str()),
            (PrismaValue::Object(t1), t2) | (t2, PrismaValue::Object(t1)) => match Self::unwrap_value(t1) {
                Some(t1) => Self::compare_values(t1, t2),
                None => left == right,
            },
            (left, right) => left == right,
        }
    }

    fn unwrap_value(obj: &[(String, PrismaValue)]) -> Option<&PrismaValue> {
        if obj.len() != 2 {
            return None;
        }

        let mut iter = obj.iter();
        let (key1, _) = iter.next().unwrap();
        let (key2, unwrapped_value) = iter.next().unwrap();

        if key1 == "$type" && key2 == "$value" {
            Some(unwrapped_value)
        } else {
            None
        }
    }
}
